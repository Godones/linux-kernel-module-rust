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
pub(crate) fn rust_helper_errname(_err: core::ffi::c_int) -> *const core::ffi::c_char {
    core::ptr::null()
}

extern "C" {
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

    #[link_name = "rust_helper_spin_lock_init"]
    pub fn spin_lock_init(
        lock: *mut spinlock_t,
        name: *const core::ffi::c_char,
        key: *mut lock_class_key,
    );
    #[link_name = "rust_helper_spin_lock"]
    pub fn spin_lock(lock: *mut spinlock_t);
    #[link_name = "rust_helper_spin_unlock"]
    pub fn spin_unlock(lock: *mut spinlock_t);

    #[link_name = "rust_helper_spin_unlock_irqrestore"]
    pub fn spin_unlock_irqrestore(lock: *mut spinlock_t, flags: core::ffi::c_ulong);
    #[link_name = "rust_helper_spin_lock_irqsave"]
    pub fn spin_lock_irqsave(lock: *mut spinlock_t) -> core::ffi::c_ulong;

    // #[link_name ="rust_helper_mutex_init"]
    // pub fn mutex_init(lock: *mut mutex);
    // #[link_name ="rust_helper_mutex_lock"]
    // pub fn mutex_lock(lock: *mut mutex);
    // #[link_name ="rust_helper_mutex_unlock"]
    // pub fn mutex_unlock(lock: *mut mutex);

    #[link_name = "rust_helper_get_current"]
    pub(crate) fn get_current() -> *mut task_struct;
    #[link_name = "rust_helper_get_task_struct"]
    pub(crate) fn get_task_struct(t: *mut task_struct);
    #[link_name = "rust_helper_put_task_struct"]
    pub(crate) fn put_task_struct(t: *mut task_struct);
    #[link_name = "rust_helper_signal_pending"]
    pub(crate) fn signal_pending(t: *mut task_struct) -> core::ffi::c_int;

    // error
    #[link_name = "rust_helper_IS_ERR"]
    pub(crate) fn is_err(ptr: *const core::ffi::c_void) -> bool_;
    #[link_name = "rust_helper_PTR_ERR"]
    pub(crate) fn ptr_err(ptr: *const core::ffi::c_void) -> core::ffi::c_long;
    // error end

    // Per-cpu
    #[link_name = "rust_helper_num_online_cpus"]
    pub(crate) fn num_online_cpus() -> core::ffi::c_uint;
    #[link_name = "rust_helper_alloc_percpu_longlong"]
    pub(crate) fn alloc_percpu_longlong() -> *mut core::ffi::c_longlong;
    #[link_name = "rust_helper_free_percpu_longlong"]
    pub(crate) fn free_percpu_longlong(p: *mut core::ffi::c_longlong);
    #[link_name = "rust_helper_get_cpu"]
    pub(crate) fn get_cpu() -> core::ffi::c_int;
    #[link_name = "rust_helper_put_cpu"]
    pub(crate) fn put_cpu();
    #[link_name = "rust_helper_per_cpu_ptr"]
    pub(crate) fn per_cpu_ptr(
        p: *mut core::ffi::c_longlong,
        cpu: core::ffi::c_int,
    ) -> *mut core::ffi::c_longlong;
    // Per-cpu end

    // Page
    #[link_name = "rust_helper_kmap"]
    pub fn kmap(page: *mut page) -> *mut core::ffi::c_void;
    #[link_name = "rust_helper_kunmap"]
    pub fn kunmap(page: *mut page);
    #[link_name = "rust_helper_kmap_atomic"]
    pub fn kmap_atomic(page: *mut page) -> *mut core::ffi::c_void;
    #[link_name = "rust_helper_kunmap_atomic"]
    pub fn kunmap_atomic(address: *mut core::ffi::c_void);
    // Page end

    // Block device
    #[link_name = "rust_helper_bio_advance_iter_single"]
    pub fn bio_advance_iter_single(bio: *const bio, iter: *mut bvec_iter, bytes: core::ffi::c_uint);
    #[link_name = "rust_helper_blk_mq_rq_to_pdu"]
    pub fn blk_mq_rq_to_pdu(rq: *mut request) -> *mut core::ffi::c_void;
    #[link_name = "rust_helper_blk_mq_rq_from_pdu"]
    pub fn blk_mq_rq_from_pdu(pdu: *mut core::ffi::c_void) -> *mut request;
    // Block device end

    // #[link_name="rust_helper_slab_is_available"]
    // pub fn slab_is_available() -> bool;

    // radix tree
    #[link_name = "rust_helper_init_radix_tree"]
    pub fn init_radix_tree(tree: *mut xarray, gfp_mask: gfp_t);
    #[link_name = "rust_helper_radix_tree_iter_init"]
    pub fn radix_tree_iter_init(
        iter: *mut radix_tree_iter,
        start: core::ffi::c_ulong,
    ) -> *mut *mut core::ffi::c_void;
    #[link_name = "rust_helper_radix_tree_next_slot"]
    pub fn radix_tree_next_slot(
        slot: *mut *mut core::ffi::c_void,
        iter: *mut radix_tree_iter,
        flags: core::ffi::c_uint,
    ) -> *mut *mut core::ffi::c_void;
}

#[repr(C)]
#[derive(Debug)]
pub(crate) struct CRcuData {
    pub data_ptr: *mut core::ffi::c_void,
}
