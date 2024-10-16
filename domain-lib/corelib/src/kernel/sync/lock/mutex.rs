// SPDX-License-Identifier: GPL-2.0

//! A kernel mutex.
//!
//! This module allows Rust code to use the kernel's `struct mutex`.

use crate::bindings;

#[macro_export]
macro_rules! new_mutex {
    ($inner:expr $(, $name:literal)? $(,)?) => {
        $crate::kernel::sync::Mutex::new(
            $inner, $crate::optional_name!($($name)?), $crate::static_lock_class!())
    };
}

pub type Mutex<T> = super::Lock<T, MutexBackend>;

/// A kernel `struct mutex` lock backend.
#[derive(Debug)]
pub struct MutexBackend;

// SAFETY: The underlying kernel `struct mutex` object ensures mutual exclusion.
unsafe impl super::Backend for MutexBackend {
    type State = bindings::mutex;
    type GuardState = ();

    unsafe fn init(
        ptr: *mut Self::State,
        name: *const core::ffi::c_char,
        key: *mut bindings::lock_class_key,
    ) {
        // SAFETY: The safety requirements ensure that `ptr` is valid for writes, and `name` and
        // `key` are valid for read indefinitely.
        crate::sys__mutex_init(ptr, name, key)
    }

    #[inline(always)]
    unsafe fn lock(ptr: *mut Self::State) -> Self::GuardState {
        // SAFETY: The safety requirements of this function ensure that `ptr` points to valid
        // memory, and that it has been initialised before.
        crate::sys_mutex_lock(ptr);
    }

    #[inline(always)]
    unsafe fn unlock(ptr: *mut Self::State, _guard_state: &Self::GuardState) {
        // SAFETY: The safety requirements of this function ensure that `ptr` is valid and that the
        // caller is the owner of the mutex.
        crate::sys_mutex_unlock(ptr);
    }
}
