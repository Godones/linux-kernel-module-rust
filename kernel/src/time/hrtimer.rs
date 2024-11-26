// SPDX-License-Identifier: GPL-2.0

//! Intrusive high resolution timers.
//!
//! # Example
//!
//! TODO

use alloc::sync::Arc;
use core::{marker::PhantomData, pin::Pin};

use crate::{
    bindings,
    init::{pin_data, *},
    types::Opaque,
};

/// # Invariants
/// `self.timer` is valid for read
#[repr(transparent)]
#[pin_data(PinnedDrop)]
pub struct Timer<T> {
    #[pin]
    timer: Opaque<bindings::hrtimer>,
    _t: PhantomData<T>,
}

impl<T: TimerCallback> Timer<T> {
    pub fn new() -> impl PinInit<Self> {
        pin_init!( Self {
            timer <- Opaque::ffi_init(move |slot: *mut bindings::hrtimer| {
                // SAFETY: By design of `pin_init!`, `slot` is a pointer live
                // allication. hrtimer_init will initialize `slot` and does not
                // require `slot` to be initialized prior to the call.
                unsafe {
                    bindings::hrtimer_init(
                        slot,
                        bindings::CLOCK_MONOTONIC as i32,
                        bindings::hrtimer_mode_HRTIMER_MODE_REL,
                    );
                }

                // SAFETY: `slot` is pointing to a live allocation, so the deref
                // is safe. The `function` field might not be initialized, but
                // `addr_of_mut` does not create a reference to the field.
                let function: *mut Option<_> = unsafe { core::ptr::addr_of_mut!((*slot).function) };

                // SAFETY: `function` points to a valid allocation.
                unsafe { core::ptr::write(function, Some(T::Receiver::run)) };
            }),
            _t: PhantomData,
        })
    }
}
// SAFETY: A `Timer` can be moved to other threads and used from there.
unsafe impl<T> Send for Timer<T> {}

// SAFETY: Timer operations are locked on C side, so it is safe to operate on a
// timer from multiple threads
unsafe impl<T> Sync for Timer<T> {}
#[pinned_drop]
impl<T> PinnedDrop for Timer<T> {
    fn drop(self: Pin<&mut Self>) {
        // SAFETY: By struct invariant `self.timer` points to a valid `struct
        // hrtimer` instance and therefore this call is safe
        unsafe {
            bindings::hrtimer_cancel(self.timer.get());
        }
    }
}

/// Implemented by structs that can use a closure to encueue itself with the timer subsystem
pub trait RawTimer: Sync {
    /// Schedule the timer after `expires` time units
    fn schedule(self, expires: u64);
}

/// Implemented by structs that contain timer nodes
pub unsafe trait HasTimer<T> {
    /// Offset of the [`Timer`] field within `Self`
    const OFFSET: usize;

    /// Return a pointer to the [`Timer`] within `Self`.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a valid struct of type `Self`.
    unsafe fn raw_get_timer(ptr: *const Self) -> *const Timer<T> {
        // SAFETY: By the safety requirement of this trait, the trait
        // implementor will have a `Timer` field at the specified offset.
        unsafe { ptr.cast::<u8>().add(Self::OFFSET).cast::<Timer<T>>() }
    }

    /// Return a pointer to the struct that is embedding the [`Timer`] pointed
    /// to by `ptr`.
    ///
    /// # Safety
    ///
    /// `ptr` must point to a [`Timer<T>`] field in a struct of type `Self`.
    unsafe fn timer_container_of(ptr: *mut Timer<T>) -> *mut Self
    where
        Self: Sized,
    {
        // SAFETY: By the safety requirement of this trait, the trait
        // implementor will have a `Timer` field at the specified offset.
        unsafe { ptr.cast::<u8>().sub(Self::OFFSET).cast::<Self>() }
    }
}

/// Implemented by structs that can be the target of a C timer callback
pub trait RawTimerCallback: RawTimer {
    unsafe extern "C" fn run(ptr: *mut bindings::hrtimer) -> bindings::hrtimer_restart;
}

/// Implemented by pointers to structs that can the target of a timer callback
pub trait TimerCallback {
    /// Type of `this` argument for `run()`.
    type Receiver: RawTimerCallback;

    /// Called by the timer logic when the timer fires
    fn run(this: Self::Receiver);
}

impl<T> RawTimer for Arc<T>
where
    T: Send + Sync,
    T: HasTimer<T>,
{
    fn schedule(self, expires: u64) {
        let self_ptr = Arc::into_raw(self);

        // SAFETY: `self_ptr` is a valid pointer to a `T`
        let timer_ptr = unsafe { T::raw_get_timer(self_ptr) };

        // `Timer` is `repr(transparent)`
        let c_timer_ptr = timer_ptr.cast::<bindings::hrtimer>();

        // Schedule the timer - if it is already scheduled it is removed and
        // inserted

        // SAFETY: c_timer_ptr points to a valid hrtimer instance that was
        // initialized by `hrtimer_init`
        unsafe {
            bindings::hrtimer_start_range_ns(
                c_timer_ptr.cast_mut(),
                expires as i64,
                0,
                bindings::hrtimer_mode_HRTIMER_MODE_REL,
            );
        }
    }
}

impl<T> RawTimerCallback for Arc<T>
where
    T: Send + Sync,
    T: HasTimer<T>,
    T: TimerCallback<Receiver = Self>,
{
    unsafe extern "C" fn run(ptr: *mut bindings::hrtimer) -> bindings::hrtimer_restart {
        // `Timer` is `repr(transparent)`
        let timer_ptr = ptr.cast::<Timer<T>>();

        // SAFETY: By C API contract `ptr` is the pointer we passed when
        // enqueing the timer, so it is a `Timer<T>` embedded in a `T`
        let data_ptr = unsafe { T::timer_container_of(timer_ptr) };

        // SAFETY: This `Arc` comes from a call to `Arc::into_raw()`
        let receiver = unsafe { Arc::from_raw(data_ptr) };

        T::run(receiver);

        bindings::hrtimer_restart_HRTIMER_NORESTART
    }
}

/// Use to implement the [`HasTimer<T>`] trait.
///
/// See [`module`] documentation for an example.
///
/// [`module`]: crate::hrtimer
#[macro_export]
macro_rules! impl_has_timer {
    ($(impl$(<$($implarg:ident),*>)?
       HasTimer<$timer_type:ty $(, $id:tt)?>
       for $self:ident $(<$($selfarg:ident),*>)?
       { self.$field:ident }
    )*) => {$(
        // SAFETY: This implementation of `raw_get_timer` only compiles if the
        // field has the right type.
        unsafe impl$(<$($implarg),*>)? $crate::time::hrtimer::HasTimer<$timer_type> for $self $(<$($selfarg),*>)? {
            const OFFSET: usize = ::core::mem::offset_of!(Self, $field) as usize;

            #[inline]
            unsafe fn raw_get_timer(ptr: *const Self) -> *const $crate::time::hrtimer::Timer<$timer_type $(, $id)?> {
                // SAFETY: The caller promises that the pointer is not dangling.
                unsafe {
                    ::core::ptr::addr_of!((*ptr).$field)
                }
            }

        }
    )*};
}
