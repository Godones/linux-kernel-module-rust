// SPDX-License-Identifier: GPL-2.0

use core::fmt::Debug;

use pinned_init::{pin_data, pin_init, try_pin_init, PinInit};

use crate::error::Error;

#[repr(align(64))]
pub struct CachePadded<T: ?Sized> {
    value: T,
}

unsafe impl<T: Send> Send for CachePadded<T> {}
unsafe impl<T: Sync> Sync for CachePadded<T> {}

impl<T> CachePadded<T> {
    /// Pads and aligns a value to 64 bytes.
    #[inline(always)]
    pub(crate) const fn new(t: T) -> CachePadded<T> {
        CachePadded::<T> { value: t }
    }
}

impl<T: ?Sized> core::ops::Deref for CachePadded<T> {
    type Target = T;

    #[inline(always)]
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: ?Sized> core::ops::DerefMut for CachePadded<T> {
    #[inline(always)]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}

impl<T: Debug> Debug for CachePadded<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CachePadded")
            .field("value", &self.value)
            .finish()
    }
}

/// Wrapper type that alings content to a cache line.
#[repr(align(64))]
#[pin_data]
pub struct CacheAligned<T: ?Sized> {
    #[pin]
    value: T,
}

impl<T> CacheAligned<T> {
    /// Pads and aligns a value to 64 bytes.
    pub const fn new(t: T) -> CacheAligned<T> {
        CacheAligned::<T> { value: t }
    }

    /// Creates an initializer for `CacheAligned<T>` form an initalizer for `T`
    pub fn new_initializer(t: impl PinInit<T>) -> impl PinInit<CacheAligned<T>> {
        pin_init!( CacheAligned {
            value <- t
        })
    }

    /// Creates a fallible initializer for `CacheAligned<T>` form a fallible
    /// initalizer for `T`
    pub fn try_new_initializer(
        t: impl PinInit<T, crate::error::Error>,
    ) -> impl PinInit<CacheAligned<T>, Error> {
        try_pin_init!( CacheAligned {
            value <- t
        }?Error)
    }

    /// Get a pointer to the contained value without creating a reference.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a valid and initialized instance of `Self` and it
    /// must be valid for read and write.
    pub const unsafe fn raw_get(ptr: *mut Self) -> *mut T {
        // SAFETY: by function safety requirements `ptr` is valid for read
        unsafe { core::ptr::addr_of_mut!((*ptr).value) }
    }
}

impl<T: ?Sized> core::ops::Deref for CacheAligned<T> {
    type Target = T;

    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: ?Sized> core::ops::DerefMut for CacheAligned<T> {
    fn deref_mut(&mut self) -> &mut T {
        &mut self.value
    }
}
