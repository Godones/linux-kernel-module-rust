use alloc::{boxed::Box, sync::Arc};
use core::{
    alloc::{AllocError, GlobalAlloc, Layout},
    ffi::c_ulong,
    mem::MaybeUninit,
    ops::Deref,
    pin::Pin,
    ptr,
    ptr::NonNull,
};

use pinned_init::{InPlaceInit, Init, PinInit};

use crate::bindings;

pub struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // krealloc is used instead of kmalloc because kmalloc is an inline function and can't be
        // bound to as a result
        if layout.size() < 4096 {
            bindings::krealloc(ptr::null(), layout.size(), bindings::GFP_KERNEL) as *mut u8
        } else {
            bindings::vzalloc(layout.size() as c_ulong) as *mut u8
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() < 4096 {
            bindings::kfree(ptr as *mut core::ffi::c_void);
        } else {
            bindings::vfree(ptr as *mut core::ffi::c_void);
        }
    }
}

impl KernelAllocator {
    pub(crate) fn allocate_with_flags(
        &self,
        layout: Layout,
        flags: bindings::gfp_t,
    ) -> Result<NonNull<[u8]>, AllocError> {
        // `krealloc()` is used instead of `kmalloc()` because the latter is
        // an inline function and cannot be bound to as a result.
        let mem = unsafe { bindings::krealloc(ptr::null(), layout.size(), flags) as *mut u8 };
        if mem.is_null() {
            return Err(AllocError);
        }
        let mem = unsafe { core::slice::from_raw_parts_mut(mem, bindings::ksize(mem as _)) };
        // Safety: checked for non null above
        Ok(unsafe { NonNull::new_unchecked(mem) })
    }
}

pub trait BoxExt<T: ?Sized> {
    fn try_new_atomic(x: T) -> Result<Self, AllocError>
    where
        Self: Sized;
}

impl<T> BoxExt<T> for Box<T> {
    fn try_new_atomic(x: T) -> Result<Box<T>, AllocError> {
        let layout = Layout::new::<MaybeUninit<T>>();
        let ptr = KernelAllocator
            .allocate_with_flags(layout, bindings::GFP_ATOMIC)?
            .cast();
        let mut boxed: Box<MaybeUninit<T>> =
            unsafe { Box::from_raw_in(ptr.as_ptr(), alloc::alloc::Global) };

        unsafe {
            boxed.as_mut_ptr().write(x);
            Ok(boxed.assume_init())
        }
    }
}

#[repr(transparent)]
pub struct UniqueArc<T: ?Sized> {
    inner: Arc<T>,
}

impl<T> InPlaceInit<T> for UniqueArc<T> {
    fn try_pin_init<E>(init: impl PinInit<T, E>) -> Result<Pin<Self>, E>
    where
        E: From<AllocError>,
    {
        let v = Arc::try_pin_init(init)?;
        let v = unsafe { core::mem::transmute(v) };
        Ok(v)
    }

    fn try_init<E>(init: impl Init<T, E>) -> Result<Self, E>
    where
        E: From<AllocError>,
    {
        let v = Arc::try_init(init)?;
        Ok(Self { inner: v })
    }
}

impl<T: ?Sized> Deref for UniqueArc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<T: ?Sized> From<Pin<UniqueArc<T>>> for Arc<T> {
    fn from(item: Pin<UniqueArc<T>>) -> Self {
        // SAFETY: The type invariants of `Arc` guarantee that the data is pinned.
        unsafe { Pin::into_inner_unchecked(item).inner }
    }
}
