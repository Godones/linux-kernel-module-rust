#![feature(allocator_api)]
#![feature(try_with_capacity)]
#![feature(const_mut_refs)]
#![feature(c_size_t)]
#![no_std]
#![allow(improper_ctypes)]
extern crate alloc;

pub mod bindings;
pub mod block;
pub mod buf;
mod build_assert;

pub mod device;
pub mod env;
pub mod error;
pub mod fs;
mod kalloc;
pub mod logger;
pub mod mm;
pub mod module;
pub mod print;
pub mod radix_tree;
pub mod random;
pub mod str;
pub mod sync;
pub mod sysctl;
mod task;
pub mod time;
pub mod types;

pub use error::linux_err as code;
pub use init::PinInit;
pub(crate) use mm::cache_padded::CachePadded;
pub use module::{param as module_param, Module, ThisModule};
/// Page size defined in terms of the `PAGE_SHIFT` macro from C.
///
/// [`PAGE_SHIFT`]: ../../../include/asm-generic/page.h
pub const PAGE_SIZE: u32 = 1 << bindings::PAGE_SHIFT;

/// Prefix to appear before log messages printed from within the `kernel` crate.
const __LOG_PREFIX: &[u8] = b"rust_kernel\0";

pub mod init {
    pub use pinned_init::*;
}
pub use kmacro::*;

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    pr_err!("Kernel panic!");
    pr_err!("{:?}", info);
    unsafe {
        bug_helper();
    }
}
extern "C" {
    fn bug_helper() -> !;
}

/// Produces a pointer to an object from a pointer to one of its fields.
///
/// # Safety
///
/// The pointer passed to this macro, and the pointer returned by this macro, must both be in
/// bounds of the same allocation.
///
/// # Examples
///
/// ```
/// # use kernel::container_of;
/// struct Test {
///     a: u64,
///     b: u32,
/// }
///
/// let test = Test { a: 10, b: 20 };
/// let b_ptr = &test.b;
/// // SAFETY: The pointer points at the `b` field of a `Test`, so the resulting pointer will be
/// // in-bounds of the same allocation as `b_ptr`.
/// let test_alias = unsafe { container_of!(b_ptr, Test, b) };
/// assert!(core::ptr::eq(&test, test_alias));
/// ```
#[macro_export]
macro_rules! container_of {
    ($ptr:expr, $type:ty, $($f:tt)*) => {{
        let ptr = $ptr as *const _ as *const u8;
        let offset: usize = ::core::mem::offset_of!($type, $($f)*);
        $crate::build_assert!(offset <= isize::MAX as usize);
        ptr.wrapping_sub(offset) as *const $type
    }}
}
