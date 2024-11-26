// SPDX-License-Identifier: GPL-2.0

//! Kernel page allocation and management.
//!
//! This module currently provides limited support. It supports pages of order 0
//! for most operations. Page allocation flags are fixed.

use core::{ ptr};
use core::alloc::AllocError;
use core::ptr::NonNull;
use crate::{bindings, code::*, error::KernelResult as Result, PAGE_SIZE};
use crate::buf::UserSliceReader;
use crate::kalloc::AllocFlags;

/// A pointer to a page that owns the page allocation.
///
/// # Invariants
///
/// The pointer is valid, and has ownership over the page.
pub struct Page {
    page: NonNull<bindings::page>,
}

// SAFETY: Pages have no logic that relies on them staying on a given thread, so moving them across
// threads is safe.
unsafe impl Send for Page {}

// SAFETY: Pages have no logic that relies on them not being accessed concurrently, so accessing
// them concurrently is safe.
unsafe impl Sync for Page {}

impl Page {
    pub fn alloc_page(flags: AllocFlags) -> Result<Self, AllocError> {
        // SAFETY: Depending on the value of `gfp_flags`, this call may sleep. Other than that, it
        // is always safe to call this method.
        let page = unsafe { bindings::alloc_pages(flags.as_raw(), 0) };
        let page = NonNull::new(page).ok_or(AllocError)?;
        // INVARIANT: We just successfully allocated a page, so we now have ownership of the newly
        // allocated page. We transfer that ownership to the new `Page` object.
        Ok(Self { page })
    }
    /// Returns a raw pointer to the page.
    pub fn as_ptr(&self) -> *mut bindings::page {
        self.page.as_ptr()
    }

    /// Create a `Page` from a raw `struct page` pointer
    ///
    /// # Safety
    ///
    /// Caller must own the page pointed to by `ptr` as these will be freed when
    /// the returned `Page` is dropped. `ptr` must point to a valid structure.
    pub unsafe fn from_raw(ptr: *mut bindings::page) -> Self {
        Self {
            // SAFETY: By function safety requirements, ptr is not null
            page: unsafe { NonNull::new_unchecked(ptr) },
        }
    }

    /// Runs a piece of code with this page mapped to an address.
    ///
    /// The page is unmapped when this call returns.
    ///
    /// # Using the raw pointer
    ///
    /// It is up to the caller to use the provided raw pointer correctly. The pointer is valid for
    /// `PAGE_SIZE` bytes and for the duration in which the closure is called. The pointer might
    /// only be mapped on the current thread, and when that is the case, dereferencing it on other
    /// threads is UB. Other than that, the usual rules for dereferencing a raw pointer apply: don't
    /// cause data races, the memory may be uninitialized, and so on.
    ///
    /// If multiple threads map the same page at the same time, then they may reference with
    /// different addresses. However, even if the addresses are different, the underlying memory is
    /// still the same for these purposes (e.g., it's still a data race if they both write to the
    /// same underlying byte at the same time).
    fn with_page_mapped<T>(&self, f: impl FnOnce(*mut u8) -> T) -> T {
        // SAFETY: `page` is valid due to the type invariants on `Page`.
        let mapped_addr = unsafe { bindings::kmap_local_page(self.as_ptr()) };

        let res = f(mapped_addr.cast());

        // This unmaps the page mapped above.
        //
        // SAFETY: Since this API takes the user code as a closure, it can only be used in a manner
        // where the pages are unmapped in reverse order. This is as required by `kunmap_local`.
        //
        // In other words, if this call to `kunmap_local` happens when a different page should be
        // unmapped first, then there must necessarily be a call to `kmap_local_page` other than the
        // call just above in `with_page_mapped` that made that possible. In this case, it is the
        // unsafe block that wraps that other call that is incorrect.
        unsafe { bindings::kunmap_local(mapped_addr) };

        res
    }
    /// Runs a piece of code with a raw pointer to a slice of this page, with bounds checking.
    ///
    /// If `f` is called, then it will be called with a pointer that points at `off` bytes into the
    /// page, and the pointer will be valid for at least `len` bytes. The pointer is only valid on
    /// this task, as this method uses a local mapping.
    ///
    /// If `off` and `len` refers to a region outside of this page, then this method returns
    /// [`EINVAL`] and does not call `f`.
    ///
    /// # Using the raw pointer
    ///
    /// It is up to the caller to use the provided raw pointer correctly. The pointer is valid for
    /// `len` bytes and for the duration in which the closure is called. The pointer might only be
    /// mapped on the current thread, and when that is the case, dereferencing it on other threads
    /// is UB. Other than that, the usual rules for dereferencing a raw pointer apply: don't cause
    /// data races, the memory may be uninitialized, and so on.
    ///
    /// If multiple threads map the same page at the same time, then they may reference with
    /// different addresses. However, even if the addresses are different, the underlying memory is
    /// still the same for these purposes (e.g., it's still a data race if they both write to the
    /// same underlying byte at the same time).
    fn with_pointer_into_page<T>(
        &self,
        off: usize,
        len: usize,
        f: impl FnOnce(*mut u8) -> Result<T>,
    ) -> Result<T> {
        let bounds_ok = off <= PAGE_SIZE as usize && len <= PAGE_SIZE as usize && (off + len) <= PAGE_SIZE as usize;

        if bounds_ok {
            self.with_page_mapped(move |page_addr| {
                // SAFETY: The `off` integer is at most `PAGE_SIZE`, so this pointer offset will
                // result in a pointer that is in bounds or one off the end of the page.
                f(unsafe { page_addr.add(off) })
            })
        } else {
            Err(EINVAL)
        }
    }
    /// Map a page into memory and run a function with a shared slice pointing
    /// to a mapped page.
    ///
    /// The page is mapped at least for the duration fo the function.
    pub fn with_slice_into_page<T>(&self, f: impl FnOnce(&[u8]) -> Result<T>) -> Result<T> {
        self.with_pointer_into_page(0, PAGE_SIZE as usize, |pointer| {
            // SAFETY:
            // * The size of the allocation pointed to by `pointer` is
            //   `PAGE_SIZE` `u8` elements.
            // * As we have a shared reference to `self` and the lifetime of
            //   this reference is captured by the returned slice, the data
            //   pointed to by `pointer` is immutable for this lifetime, and
            //   thus valid for reads.
            // * `pointer` points to aligned `u8` data, because alignment of `u8` is 1.
            // * The size of the returned slice is less than `isize::MAX`
            //   because it is bounded by `PAGE_SIZE`.
            let buffer =
                unsafe { core::slice::from_raw_parts(pointer.cast::<u8>(), PAGE_SIZE as usize) };
            f(buffer)
        })
    }

    /// Map a page into memory and run a function with a mutable slice pointing
    /// to a mapped page.
    ///
    /// The page is mapped at least for the duration fo the function.
    pub fn with_slice_into_page_mut<T>(
        &mut self,
        f: impl FnOnce(&mut [u8]) -> Result<T>,
    ) -> Result<T> {
        self.with_pointer_into_page(0, PAGE_SIZE as usize, |pointer| {
            // SAFETY:
            // * The size of the allocation pointed to by `pointer` is
            //   `PAGE_SIZE` `u8` elements.
            // * As we have a mutable reference to `self` and the lifetime of
            //   this reference is captured by the returned slice, we have
            //   exclusive access to the data pointed to by `pointer` for this
            //   lifetime, and the data is valid for read and write.
            // * `pointer` points to aligned `u8` data, because alignment of `u8` is 1.
            // * The size of the returned slice is less than `isize::MAX`
            //   because it is bounded by `PAGE_SIZE`.
            let buffer = unsafe {
                core::slice::from_raw_parts_mut(pointer.cast::<u8>(), PAGE_SIZE as usize)
            };
            f(buffer)
        })
    }
    /// Maps the page and reads from it into the given buffer.
    ///
    /// This method will perform bounds checks on the page offset. If `offset .. offset+len` goes
    /// outside of the page, then this call returns [`EINVAL`].
    ///
    /// # Safety
    ///
    /// * Callers must ensure that `dst` is valid for writing `len` bytes.
    /// * Callers must ensure that this call does not race with a write to the same page that
    ///   overlaps with this read.
    pub unsafe fn read_raw(&self, dst: *mut u8, offset: usize, len: usize) -> Result {
        self.with_pointer_into_page(offset, len, move |src| {
            // SAFETY: If `with_pointer_into_page` calls into this closure, then
            // it has performed a bounds check and guarantees that `src` is
            // valid for `len` bytes.
            //
            // There caller guarantees that there is no data race.
            unsafe { ptr::copy_nonoverlapping(src, dst, len) };
            Ok(())
        })
    }

    /// Maps the page and writes into it from the given buffer.
    ///
    /// This method will perform bounds checks on the page offset. If `offset .. offset+len` goes
    /// outside of the page, then this call returns [`EINVAL`].
    ///
    /// # Safety
    ///
    /// * Callers must ensure that `src` is valid for reading `len` bytes.
    /// * Callers must ensure that this call does not race with a read or write to the same page
    ///   that overlaps with this write.
    pub unsafe fn write_raw(&self, src: *const u8, offset: usize, len: usize) -> Result {
        self.with_pointer_into_page(offset, len, move |dst| {
            // SAFETY: If `with_pointer_into_page` calls into this closure, then it has performed a
            // bounds check and guarantees that `dst` is valid for `len` bytes.
            //
            // There caller guarantees that there is no data race.
            unsafe { ptr::copy_nonoverlapping(src, dst, len) };
            Ok(())
        })
    }

    /// Maps the page and zeroes the given slice.
    ///
    /// This method will perform bounds checks on the page offset. If `offset .. offset+len` goes
    /// outside of the page, then this call returns [`EINVAL`].
    ///
    /// # Safety
    ///
    /// Callers must ensure that this call does not race with a read or write to the same page that
    /// overlaps with this write.
    pub unsafe fn fill_zero_raw(&self, offset: usize, len: usize) -> Result {
        self.with_pointer_into_page(offset, len, move |dst| {
            // SAFETY: If `with_pointer_into_page` calls into this closure, then it has performed a
            // bounds check and guarantees that `dst` is valid for `len` bytes.
            //
            // There caller guarantees that there is no data race.
            unsafe { ptr::write_bytes(dst, 0u8, len) };
            Ok(())
        })
    }

    /// Copies data from userspace into this page.
    ///
    /// This method will perform bounds checks on the page offset. If `offset .. offset+len` goes
    /// outside of the page, then this call returns [`EINVAL`].
    ///
    /// Like the other `UserSliceReader` methods, data races are allowed on the userspace address.
    /// However, they are not allowed on the page you are copying into.
    ///
    /// # Safety
    ///
    /// Callers must ensure that this call does not race with a read or write to the same page that
    /// overlaps with this write.
    pub unsafe fn copy_from_user_slice_raw(
        &self,
        reader: &mut UserSliceReader,
        offset: usize,
        len: usize,
    ) -> Result {
        self.with_pointer_into_page(offset, len, move |dst| {
            // SAFETY: If `with_pointer_into_page` calls into this closure, then it has performed a
            // bounds check and guarantees that `dst` is valid for `len` bytes. Furthermore, we have
            // exclusive access to the slice since the caller guarantees that there are no races.
            reader.read_raw(unsafe { core::slice::from_raw_parts_mut(dst.cast(), len) })
        })
    }
}
impl Drop for Page {
    fn drop(&mut self) {
        // SAFETY: By the type invariants, we have ownership of the page and can free it.
        unsafe { bindings::__free_pages(self.page.as_ptr(), 0) };
    }
}
