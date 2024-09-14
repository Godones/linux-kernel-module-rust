#[allow(
    clippy::all,
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    improper_ctypes
)]
mod bindings {
    use crate::c_types;
    include!("bindings_c.rs");
}
pub use bindings::*;

pub const GFP_KERNEL: gfp_t = BINDINGS_GFP_KERNEL;
extern "C" {
    pub fn _printk(fmt: *const core::ffi::c_char, ...) -> core::ffi::c_int;
}
