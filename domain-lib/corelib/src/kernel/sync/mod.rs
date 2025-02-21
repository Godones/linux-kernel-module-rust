use alloc::{alloc::Global, sync::Arc};
use core::{
    alloc::{AllocError, Allocator},
    ops::Deref,
    pin::Pin,
};

use pinned_init::{InPlaceInit, InPlaceInitIn, Init, PinInit};

use crate::{bindings, kernel::types::Opaque};
mod lock;
pub mod rcu;
mod revocable;
pub use lock::{mutex::Mutex, spinlock::SpinLock};
pub use revocable::RevocableMutex;

/// Represents a lockdep class. It's a wrapper around C's `lock_class_key`.
#[repr(transparent)]
pub struct LockClassKey(Opaque<bindings::lock_class_key>);

// SAFETY: `bindings::lock_class_key` is designed to be used concurrently from multiple threads and
// provides its own synchronization.
unsafe impl Sync for LockClassKey {}

impl LockClassKey {
    /// Creates a new lock class key.
    pub const fn new() -> Self {
        Self(Opaque::uninit())
    }

    pub(crate) fn as_ptr(&self) -> *mut bindings::lock_class_key {
        self.0.get()
    }
}

/// Defines a new static lock class and returns a pointer to it.
#[doc(hidden)]
#[macro_export]
macro_rules! static_lock_class {
    () => {{
        static CLASS: $crate::kernel::sync::LockClassKey =
            $crate::kernel::sync::LockClassKey::new();
        &CLASS
    }};
}

/// Returns the given string, if one is provided, otherwise generates one based on the source code
/// location.
#[doc(hidden)]
#[macro_export]
macro_rules! optional_name {
    () => {
        $crate::c_str!(::core::concat!(::core::file!(), ":", ::core::line!()))
    };
    ($name:literal) => {
        $crate::c_str!($name)
    };
}

#[repr(transparent)]
pub struct UniqueArc<T: ?Sized, A: Allocator = Global> {
    inner: Arc<T, A>,
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

impl<T, A: Allocator> InPlaceInitIn<T, A> for UniqueArc<T, A> {
    fn try_pin_init_in<E>(init: impl PinInit<T, E>, alloc: A) -> Result<Pin<Self>, E>
    where
        E: From<AllocError>,
    {
        let v = Arc::try_pin_init_in(init, alloc)?;
        unsafe {
            let inner = Pin::into_inner_unchecked(v);
            let r = Pin::new_unchecked(UniqueArc { inner });
            Ok(r)
        }
    }
    fn try_init_in<E>(init: impl Init<T, E>, alloc: A) -> Result<Self, E>
    where
        E: From<AllocError>,
    {
        let v = Arc::try_init_in(init, alloc)?;
        Ok(Self { inner: v })
    }
}

impl<T: ?Sized, A: Allocator> Deref for UniqueArc<T, A> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.inner.deref()
    }
}

impl<T: ?Sized, A: Allocator> From<Pin<UniqueArc<T, A>>> for Arc<T, A> {
    fn from(item: Pin<UniqueArc<T, A>>) -> Self {
        // SAFETY: The type invariants of `Arc` guarantee that the data is pinned.
        unsafe { Pin::into_inner_unchecked(item).inner }
    }
}
