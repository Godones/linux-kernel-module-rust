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
pub mod chrdev;
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
