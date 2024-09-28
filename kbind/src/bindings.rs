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
    pub(crate) fn rust_helper_errname(err: core::ffi::c_int) -> *const core::ffi::c_char;
    pub(crate) fn rust_helper_rcu_read_unlock();
    pub(crate) fn rust_helper_rcu_read_lock();
    pub(crate) fn rust_helper_synchronize_rcu();
    pub(crate) fn rust_helper_rcu_assign_pointer(
        rcu_data: *mut CRcuData,
        new_ptr: *const core::ffi::c_void,
    );
    pub(crate) fn rust_helper_rcu_dereference(
        rcu_data: *const CRcuData,
    ) -> *const core::ffi::c_void;

    pub(crate) fn rust_helper_spin_lock_init(lock: *mut spinlock_t);
    pub(crate) fn rust_helper_spin_lock(lock: *mut spinlock_t);
    pub(crate) fn rust_helper_spin_unlock(lock: *mut spinlock_t);
    pub(crate) fn rust_helper_mutex_init(lock: *mut mutex);
    pub(crate) fn rust_helper_mutex_lock(lock: *mut mutex);
    pub(crate) fn rust_helper_mutex_unlock(lock: *mut mutex);
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct CRcuData {
    pub data_ptr: *mut core::ffi::c_void,
}
