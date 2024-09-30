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

extern "C" {
    pub(crate) fn rust_helper_errname(err: core::ffi::c_int) -> *const core::ffi::c_char;
    pub(crate) fn rust_helper_rcu_read_unlock();
    pub(crate) fn rust_helper_rcu_read_lock();
    pub(crate) fn rust_helper_synchronize_rcu();
    pub(crate) fn rust_helper_rcu_assign_pointer(
        rcu_data: *const CRcuData,
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

    pub(crate) fn rust_helper_get_current() -> *mut task_struct;
    pub(crate) fn rust_helper_get_task_struct(t: *mut task_struct);
    pub(crate) fn rust_helper_put_task_struct(t: *mut task_struct);
    pub(crate) fn rust_helper_signal_pending(t: *mut task_struct) -> core::ffi::c_int;

    pub(crate) fn rust_helper_IS_ERR(ptr: *const core::ffi::c_void) -> bool_;
    pub(crate) fn rust_helper_PTR_ERR(ptr: *const core::ffi::c_void) -> core::ffi::c_long;

    pub(crate) fn rust_helper_blk_mq_rq_to_pdu(rq: *mut request) -> *mut core::ffi::c_void;

    pub(crate) fn rust_helper_blk_mq_rq_from_pdu(pdu: *mut core::ffi::c_void) -> *mut request;

    pub(crate) fn rust_helper_num_online_cpus() -> core::ffi::c_uint;

    pub(crate) fn rust_helper_alloc_percpu_longlong() -> *mut core::ffi::c_longlong;

    pub(crate) fn rust_helper_free_percpu_longlong(p: *mut core::ffi::c_longlong);

    pub(crate) fn rust_helper_get_cpu() -> core::ffi::c_int;

    pub(crate) fn rust_helper_put_cpu();
    pub(crate) fn rust_helper_per_cpu_ptr(
        p: *mut core::ffi::c_longlong,
        cpu: core::ffi::c_int,
    ) -> *mut core::ffi::c_longlong;
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct CRcuData {
    pub data_ptr: *mut core::ffi::c_void,
}

pub(crate) unsafe fn is_err(ptr: *const core::ffi::c_void) -> bool {
    rust_helper_IS_ERR(ptr)
}
pub(crate) unsafe fn ptr_err(ptr: *const core::ffi::c_void) -> core::ffi::c_long {
    rust_helper_PTR_ERR(ptr)
}
pub(crate) unsafe fn get_current() -> *mut task_struct {
    rust_helper_get_current()
}
pub(crate) unsafe fn get_task_struct(t: *mut task_struct) {
    rust_helper_get_task_struct(t)
}
pub(crate) unsafe fn put_task_struct(t: *mut task_struct) {
    rust_helper_put_task_struct(t)
}

pub(crate) unsafe fn signal_pending(t: *mut task_struct) -> core::ffi::c_int {
    rust_helper_signal_pending(t)
}

pub(crate) unsafe fn blk_mq_rq_to_pdu(rq: *mut request) -> *mut core::ffi::c_void {
    rust_helper_blk_mq_rq_to_pdu(rq)
}

pub(crate) unsafe fn blk_mq_rq_from_pdu(pdu: *mut core::ffi::c_void) -> *mut request {
    rust_helper_blk_mq_rq_from_pdu(pdu)
}

#[inline]
pub(crate) unsafe fn num_online_cpus() -> core::ffi::c_uint {
    rust_helper_num_online_cpus()
}

/// dynamically allocate and free per-cpu variables with long long type
#[inline]
pub(crate) unsafe fn alloc_percpu_longlong() -> *mut core::ffi::c_longlong {
    rust_helper_alloc_percpu_longlong()
}

/// dynamically allocate and free per-cpu variables with long long type
#[inline]
pub(crate) unsafe fn free_percpu_longlong(p: *mut core::ffi::c_longlong) {
    rust_helper_free_percpu_longlong(p)
}

/// get current cpu
#[inline]
pub(crate) unsafe fn get_cpu() -> core::ffi::c_int {
    rust_helper_get_cpu()
}

/// put current cpu
#[inline]
pub(crate) unsafe fn put_cpu() {
    rust_helper_put_cpu()
}

/// get per-cpu pointer with long long type
#[inline]
pub(crate) unsafe fn per_cpu_ptr(
    p: *mut core::ffi::c_longlong,
    cpu: core::ffi::c_int,
) -> *mut core::ffi::c_longlong {
    rust_helper_per_cpu_ptr(p, cpu)
}
