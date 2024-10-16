// SPDX-License-Identifier: GPL-2.0

//! A kernel spinlock.
//!
//! This module allows Rust code to use the kernel's `spinlock_t`.

use core::ops::DerefMut;

use crate::{bindings, kernel::mm::cache_padded::CachePadded};

/// Creates a [`SpinLock`] initialiser with the given name and a newly-created lock class.
///
/// It uses the name if one is given, otherwise it generates one based on the file name and line
/// number.
#[macro_export]
macro_rules! new_spinlock {
    ($inner:expr $(, $name:literal)? $(,)?) => {
        $crate::kernel::sync::SpinLock::new(
            $inner, $crate::optional_name!($($name)?), $crate::static_lock_class!())
    };
}

pub type SpinLock<T> = super::Lock<T, SpinLockBackend>;

/// A kernel `spinlock_t` lock backend.
#[derive(Debug)]
pub struct SpinLockBackend;

// SAFETY: The underlying kernel `spinlock_t` object ensures mutual exclusion. `relock` uses the
// default implementation that always calls the same locking method.
unsafe impl super::Backend for SpinLockBackend {
    type State = CachePadded<bindings::spinlock_t>;
    type GuardState = Option<core::ffi::c_ulong>;

    unsafe fn init(
        ptr: *mut Self::State,
        name: *const core::ffi::c_char,
        key: *mut bindings::lock_class_key,
    ) {
        // SAFETY: The safety requirements ensure that `ptr` is valid for writes, and `name` and
        // `key` are valid for read indefinitely.
        crate::sys_spin_lock_init((&mut *ptr).deref_mut(), name, key)
    }

    #[inline(always)]
    unsafe fn lock(ptr: *mut Self::State) -> Self::GuardState {
        // SAFETY: The safety requirements of this function ensure that `ptr` points to valid
        // memory, and that it has been initialised before.
        crate::sys_spin_lock((&mut *ptr).deref_mut());
        None
    }

    #[inline(always)]
    unsafe fn unlock(ptr: *mut Self::State, guard_state: &Self::GuardState) {
        match guard_state {
            // SAFETY: The safety requirements of this function ensure that `ptr` is valid and that
            // the caller is the owner of the mutex.
            Some(flags) =>
                crate::sys_spin_unlock_irqrestore((&mut *ptr).deref_mut(), *flags)
            ,
            // SAFETY: The safety requirements of this function ensure that `ptr` is valid and that
            // the caller is the owner of the mutex.
            None =>  crate::sys_spin_unlock((&mut *ptr).deref_mut()) ,
        }
    }
}

// SAFETY: The underlying kernel `spinlock_t` object ensures mutual exclusion. We use the `irqsave`
// variant of the C lock acquisition functions to disable interrupts and retrieve the original
// interrupt state, and the `irqrestore` variant of the lock release functions to restore the state
// in `unlock` -- we use the guard context to determine which method was used to acquire the lock.
unsafe impl super::IrqSaveBackend for SpinLockBackend {
    #[inline(always)]
    unsafe fn lock_irqsave(ptr: *mut Self::State) -> Self::GuardState {
        // SAFETY: The safety requirements of this function ensure that `ptr` points to valid
        // memory, and that it has been initialised before.
        Some(crate::sys_spin_lock_irqsave((&mut *ptr).deref_mut()))
    }
}
