#![no_std]
#[allow(
    clippy::all,
    missing_docs,
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    improper_ctypes,
    unreachable_pub,
    unsafe_op_in_unsafe_fn
)]

mod bindings {
    include!("bindings_c.rs");
}
pub use bindings::*;

pub const GFP_KERNEL: gfp_t = BINDINGS_GFP_KERNEL;
pub const BINDINGS_GFP_ATOMIC: gfp_t = 2080;
pub const BINDINGS___GFP_ZERO: gfp_t = 256;
pub const GFP_ATOMIC: gfp_t = BINDINGS_GFP_ATOMIC;
pub const __GFP_ZERO: gfp_t = BINDINGS___GFP_ZERO;

// wait to remove
pub const SLAB_RECLAIM_ACCOUNT: slab_flags_t = 32768;
pub const SLAB_ACCOUNT: slab_flags_t = 8192;
pub const MAX_LFS_FILESIZE: loff_t = 9223372036854775807;
pub const PAGE_SIZE: usize = 4096;
pub const SB_RDONLY: core::ffi::c_ulong = 1;
