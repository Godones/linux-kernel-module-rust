#[allow(
    clippy::all,
    non_camel_case_types,
    non_upper_case_globals,
    non_snake_case,
    improper_ctypes
)]
mod bindings {
    include!("bindings_c.rs");
}
pub use bindings::*;

pub const GFP_KERNEL: gfp_t = BINDINGS_GFP_KERNEL;

extern "C" {
    #[link_name = "rust_helper_errname"]
    pub fn errname(err: core::ffi::c_int) -> *const core::ffi::c_char;
}
