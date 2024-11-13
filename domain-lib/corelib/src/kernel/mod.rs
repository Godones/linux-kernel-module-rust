use kbind::safe_ptr::SafePtr;

use crate::bindings;

pub mod block;
pub mod device;
pub mod driver;
pub mod error;
pub mod irq;
pub mod mm;
pub mod pci;
pub mod radix_tree;
pub mod revocable;
pub mod str;
pub mod sync;
pub mod time;
pub mod types;

/// Equivalent to `THIS_MODULE` in the C API.
///
/// C header: `include/linux/export.h`
#[repr(transparent)]
pub struct ThisModule(*mut bindings::module);

// SAFETY: `THIS_MODULE` may be used from all threads within a module.
unsafe impl Sync for ThisModule {}

impl ThisModule {
    /// Creates a [`ThisModule`] given the `THIS_MODULE` pointer.
    ///
    /// # Safety
    ///
    /// The pointer must be equal to the right `THIS_MODULE`.
    pub const unsafe fn from_ptr(ptr: *mut bindings::module) -> ThisModule {
        ThisModule(ptr)
    }

    pub fn as_ptr(&self) -> *mut bindings::module {
        self.0
    }

    pub fn from_safe_ptr(ptr: SafePtr) -> ThisModule {
        unsafe { ThisModule::from_ptr(ptr.raw_ptr() as *mut bindings::module) }
    }
}

#[macro_export]
macro_rules! build_assert {
    ($cond:expr $(,)?) => {{
        if !$cond {
            $crate::kernel::error::build_error(concat!("assertion failed: ", stringify!($cond)));
        }
    }};
    ($cond:expr, $msg:expr) => {{
        if !$cond {
            $crate::kernel::error::build_error($msg);
        }
    }};
}

#[macro_export]
macro_rules! static_assert {
    ($condition:expr) => {
        const _: () = core::assert!($condition);
    };
}
