#![feature(allocator_api)]
#![feature(try_with_capacity)]
#![allow(non_snake_case)]
#![no_std]
extern crate alloc;

use alloc::sync::Arc;
use core::any::Any;

#[cfg(feature = "core_impl")]
pub use core_impl::*;
use interface::{DomainType, DomainTypeRaw};
pub use pconst::LinuxErrno;
use spin::Once;

pub mod bindings;
pub mod domain_info;
pub mod kernel;

pub type LinuxResult<T> = Result<T, LinuxErrno>;
pub type LinuxError = LinuxErrno;

use bindings::*;
pub trait CoreFunction: Send + Sync {
    fn sys_alloc_pages(&self, domain_id: u64, n: usize) -> *mut u8;
    fn sys_free_pages(&self, domain_id: u64, p: *mut u8, n: usize);
    fn sys_write_console(&self, s: &str);
    fn sys_backtrace(&self, domain_id: u64);
    /// This func will be deleted
    fn blk_crash_trick(&self) -> bool;
    fn sys_get_domain(&self, name: &str) -> Option<DomainType>;
    fn sys_create_domain(
        &self,
        domain_file_name: &str,
        identifier: &mut [u8],
    ) -> LinuxResult<DomainType>;
    /// Register a new domain with the given name and type
    fn sys_register_domain(&self, ident: &str, ty: DomainTypeRaw, data: &[u8]) -> LinuxResult<()>;
    /// Replace the old domain with the new domain
    fn sys_update_domain(
        &self,
        old_domain_name: &str,
        new_domain_name: &str,
        ty: DomainTypeRaw,
    ) -> LinuxResult<()>;
    fn sys_reload_domain(&self, domain_name: &str) -> LinuxResult<()>;
    fn checkout_shared_data(&self) -> LinuxResult<()>;
    fn domain_info(&self) -> LinuxResult<Arc<dyn Any + Send + Sync>>;

    // linux kernel func list
    fn sys_err_ptr(&self, err: core::ffi::c_long) -> *mut core::ffi::c_void;
    fn sys_is_err(&self, ptr: *const core::ffi::c_void) -> bool;
    fn sys_ptr_err(&self, ptr: *const core::ffi::c_void) -> core::ffi::c_long;
    fn sys_errno_to_blk_status(&self, errno: core::ffi::c_int) -> blk_status_t;
    fn sys_bio_advance_iter_single(
        &self,
        bio: *const bio,
        iter: *mut bvec_iter,
        bytes: core::ffi::c_uint,
    );
    fn sys_kmap(&self, page: *mut page) -> *mut core::ffi::c_void;
    fn sys_kunmap(&self, page: *mut page);
    fn sys_kmap_atomic(&self, page: *mut page) -> *mut core::ffi::c_void;
    fn sys_kunmap_atomic(&self, address: *mut core::ffi::c_void);
    fn sys__alloc_pages(&self, gfp: gfp_t, order: core::ffi::c_uint) -> *mut page;
    fn sys__free_pages(&self, page: *mut page, order: core::ffi::c_uint);

    fn sys__blk_mq_alloc_disk(
        &self,
        set: *mut blk_mq_tag_set,
        queuedata: *mut core::ffi::c_void,
        lkclass: *mut lock_class_key,
    ) -> *mut gendisk;
    fn sys_device_add_disk(
        &self,
        parent: *mut device,
        disk: *mut gendisk,
        groups: *mut *const attribute_group,
    ) -> core::ffi::c_int;
    fn sys_set_capacity(&self, disk: *mut gendisk, size: sector_t);
    fn sys_blk_queue_logical_block_size(&self, arg1: *mut request_queue, arg2: core::ffi::c_uint);
    fn sys_blk_queue_physical_block_size(&self, arg1: *mut request_queue, arg2: core::ffi::c_uint);
    fn sys_blk_queue_flag_set(&self, flag: core::ffi::c_uint, q: *mut request_queue);
    fn sys_blk_queue_flag_clear(&self, flag: core::ffi::c_uint, q: *mut request_queue);
    fn sys_del_gendisk(&self, disk: *mut gendisk);
    fn sys_blk_mq_rq_to_pdu(&self, rq: *mut request) -> *mut core::ffi::c_void;
    fn sys_blk_mq_start_request(&self, rq: *mut request);
    fn sys_blk_mq_end_request(&self, rq: *mut request, status: blk_status_t);
    fn sys_blk_mq_complete_request_remote(&self, rq: *mut request) -> bool;
    fn sys_blk_mq_rq_from_pdu(&self, pdu: *mut core::ffi::c_void) -> *mut request;
    fn sys_blk_mq_alloc_tag_set(&self, set: *mut blk_mq_tag_set) -> core::ffi::c_int;
    fn sys_blk_mq_free_tag_set(&self, set: *mut blk_mq_tag_set);

    // mutex
    fn sys__mutex_init(
        &self,
        ptr: *mut mutex,
        name: *const core::ffi::c_char,
        key: *mut lock_class_key,
    );
    fn sys_mutex_lock(&self, ptr: *mut mutex);
    fn sys_mutex_unlock(&self, ptr: *mut mutex);

    fn sys_spin_lock_init(
        &self,
        ptr: *mut spinlock_t,
        name: *const core::ffi::c_char,
        key: *mut lock_class_key,
    );
    fn sys_spin_lock(&self, ptr: *mut spinlock_t);
    fn sys_spin_unlock(&self, ptr: *mut spinlock_t);
    fn sys_spin_lock_irqsave(&self, lock: *mut spinlock_t) -> core::ffi::c_ulong;
    fn sys_spin_unlock_irqrestore(&self, lock: *mut spinlock_t, flags: core::ffi::c_ulong);

    // tree
    fn sys_init_radix_tree(&self, tree: *mut xarray, gfp_mask: gfp_t);
    fn sys_radix_tree_insert(
        &self,
        arg1: *mut xarray,
        index: core::ffi::c_ulong,
        arg2: *mut core::ffi::c_void,
    ) -> core::ffi::c_int;
    fn sys_radix_tree_lookup(
        &self,
        arg1: *const xarray,
        arg2: core::ffi::c_ulong,
    ) -> *mut core::ffi::c_void;
    fn sys_radix_tree_delete(
        &self,
        arg1: *mut xarray,
        arg2: core::ffi::c_ulong,
    ) -> *mut core::ffi::c_void;
    fn sys_radix_tree_iter_init(
        &self,
        iter: *mut radix_tree_iter,
        start: core::ffi::c_ulong,
    ) -> *mut *mut core::ffi::c_void;
    fn sys_radix_tree_next_chunk(
        &self,
        arg1: *const xarray,
        iter: *mut radix_tree_iter,
        flags: core::ffi::c_uint,
    ) -> *mut *mut core::ffi::c_void;
    fn sys_radix_tree_next_slot(
        &self,
        slot: *mut *mut core::ffi::c_void,
        iter: *mut radix_tree_iter,
        flags: core::ffi::c_uint,
    ) -> *mut *mut core::ffi::c_void;

    // time
    fn sys_hrtimer_init(&self, timer: *mut hrtimer, which_clock: clockid_t, mode: hrtimer_mode);
    fn sys_hrtimer_cancel(&self, timer: *mut hrtimer) -> core::ffi::c_int;
    fn sys_hrtimer_start_range_ns(
        &self,
        timer: *mut hrtimer,
        tim: ktime_t,
        range_ns: u64_,
        mode: hrtimer_mode,
    );
}

#[cfg(feature = "core_impl")]
mod core_impl {
    use alloc::sync::Arc;
    use core::any::Any;

    use bindings::*;
    use interface::{DomainType, DomainTypeRaw};
    use kbind::blk_status_t;
    use spin::Once;

    use super::{bindings, LinuxError, LinuxResult, OnceGet};
    use crate::CoreFunction;

    static CORE_FUNC: Once<&'static dyn CoreFunction> = Once::new();

    extern "C" {
        fn sbss();
        fn ebss();
    }
    fn clear_bss() {
        unsafe {
            core::slice::from_raw_parts_mut(
                sbss as usize as *mut u8,
                ebss as usize - sbss as usize,
            )
            .fill(0);
        }
    }

    pub fn init(syscall: &'static dyn CoreFunction) {
        clear_bss();
        CORE_FUNC.call_once(|| syscall);
    }

    pub fn alloc_raw_pages(n: usize, domain_id: u64) -> *mut u8 {
        CORE_FUNC.get_must().sys_alloc_pages(domain_id, n)
    }

    pub fn free_raw_pages(p: *mut u8, n: usize, domain_id: u64) {
        CORE_FUNC.get_must().sys_free_pages(domain_id, p, n);
    }

    pub fn write_console(s: &str) {
        CORE_FUNC.get_must().sys_write_console(s);
    }

    pub fn backtrace(domain_id: u64) {
        CORE_FUNC.get_must().sys_backtrace(domain_id);
    }

    // todo!(delete)
    pub fn blk_crash_trick() -> bool {
        CORE_FUNC.get_must().blk_crash_trick()
    }

    pub fn get_domain(name: &str) -> Option<DomainType> {
        CORE_FUNC.get_must().sys_get_domain(name)
    }

    pub fn create_domain(
        domain_file_name: &str,
        domain_identifier: &mut [u8],
    ) -> LinuxResult<DomainType> {
        if domain_identifier.len() < 32 {
            return Err(LinuxError::EINVAL);
        }
        CORE_FUNC
            .get_must()
            .sys_create_domain(domain_file_name, domain_identifier)
    }

    pub fn register_domain(ident: &str, ty: DomainTypeRaw, data: &[u8]) -> LinuxResult<()> {
        CORE_FUNC.get_must().sys_register_domain(ident, ty, data)
    }

    pub fn update_domain(
        old_domain_name: &str,
        new_domain_name: &str,
        ty: DomainTypeRaw,
    ) -> LinuxResult<()> {
        CORE_FUNC
            .get_must()
            .sys_update_domain(old_domain_name, new_domain_name, ty)
    }

    pub fn reload_domain(domain_name: &str) -> LinuxResult<()> {
        CORE_FUNC.get_must().sys_reload_domain(domain_name)
    }
    pub fn checkout_shared_data() -> LinuxResult<()> {
        CORE_FUNC.get_must().checkout_shared_data()
    }

    pub fn domain_info() -> LinuxResult<Arc<dyn Any + Send + Sync>> {
        CORE_FUNC.get_must().domain_info()
    }

    // kernel binding func
    pub(crate) fn sys_err_ptr(err: core::ffi::c_long) -> *mut core::ffi::c_void {
        CORE_FUNC.get_must().sys_err_ptr(err)
    }
    pub(crate) fn sys_errname(_err: core::ffi::c_int) -> *const core::ffi::c_char {
        core::ptr::null()
    }
    pub(crate) fn sys_is_err(ptr: *const core::ffi::c_void) -> bool {
        CORE_FUNC.get_must().sys_is_err(ptr)
    }
    pub(crate) fn sys_ptr_err(ptr: *const core::ffi::c_void) -> core::ffi::c_long {
        CORE_FUNC.get_must().sys_ptr_err(ptr)
    }
    pub(crate) fn sys_errno_to_blk_status(errno: core::ffi::c_int) -> blk_status_t {
        CORE_FUNC.get_must().sys_errno_to_blk_status(errno)
    }
    pub(crate) fn sys_bio_advance_iter_single(
        bio: *const bio,
        iter: *mut bvec_iter,
        bytes: core::ffi::c_uint,
    ) {
        CORE_FUNC
            .get_must()
            .sys_bio_advance_iter_single(bio, iter, bytes)
    }
    pub(crate) fn sys_kmap(page: *mut page) -> *mut core::ffi::c_void {
        CORE_FUNC.get_must().sys_kmap(page)
    }
    pub(crate) fn sys__alloc_pages(gfp: gfp_t, order: core::ffi::c_uint) -> *mut page {
        CORE_FUNC.get_must().sys__alloc_pages(gfp, order)
    }
    pub(crate) fn sys_kmap_atomic(page: *mut page) -> *mut core::ffi::c_void {
        CORE_FUNC.get_must().sys_kmap_atomic(page)
    }
    pub(crate) fn sys__free_pages(page: *mut page, order: core::ffi::c_uint) {
        CORE_FUNC.get_must().sys__free_pages(page, order)
    }
    pub(crate) fn sys_kunmap_atomic(address: *mut core::ffi::c_void) {
        CORE_FUNC.get_must().sys_kunmap_atomic(address)
    }
    pub(crate) fn sys_kunmap(page: *mut page) {
        CORE_FUNC.get_must().sys_kunmap(page)
    }
    pub(crate) fn sys__blk_mq_alloc_disk(
        set: *mut blk_mq_tag_set,
        queuedata: *mut core::ffi::c_void,
        lkclass: *mut lock_class_key,
    ) -> *mut gendisk {
        CORE_FUNC
            .get_must()
            .sys__blk_mq_alloc_disk(set, queuedata, lkclass)
    }

    pub(crate) fn sys_device_add_disk(
        parent: *mut device,
        disk: *mut gendisk,
        groups: *mut *const attribute_group,
    ) -> core::ffi::c_int {
        CORE_FUNC
            .get_must()
            .sys_device_add_disk(parent, disk, groups)
    }
    pub(crate) fn sys_set_capacity(disk: *mut gendisk, size: sector_t) {
        CORE_FUNC.get_must().sys_set_capacity(disk, size)
    }

    pub(crate) fn sys_blk_queue_logical_block_size(
        arg1: *mut request_queue,
        arg2: core::ffi::c_uint,
    ) {
        CORE_FUNC
            .get_must()
            .sys_blk_queue_logical_block_size(arg1, arg2)
    }
    pub(crate) fn sys_blk_queue_physical_block_size(
        arg1: *mut request_queue,
        arg2: core::ffi::c_uint,
    ) {
        CORE_FUNC
            .get_must()
            .sys_blk_queue_physical_block_size(arg1, arg2)
    }
    pub(crate) fn sys_blk_queue_flag_set(flag: core::ffi::c_uint, q: *mut request_queue) {
        CORE_FUNC.get_must().sys_blk_queue_flag_set(flag, q)
    }
    pub(crate) fn sys_blk_queue_flag_clear(flag: core::ffi::c_uint, q: *mut request_queue) {
        CORE_FUNC.get_must().sys_blk_queue_flag_clear(flag, q)
    }
    pub(crate) fn sys_del_gendisk(disk: *mut gendisk) {
        CORE_FUNC.get_must().sys_del_gendisk(disk)
    }
    pub(crate) fn sys_blk_mq_rq_to_pdu(rq: *mut request) -> *mut core::ffi::c_void {
        CORE_FUNC.get_must().sys_blk_mq_rq_to_pdu(rq)
    }
    pub(crate) fn sys_blk_mq_start_request(rq: *mut request) {
        CORE_FUNC.get_must().sys_blk_mq_start_request(rq)
    }
    pub(crate) fn sys_blk_mq_end_request(rq: *mut request, status: blk_status_t) {
        CORE_FUNC.get_must().sys_blk_mq_end_request(rq, status)
    }
    pub(crate) fn sys_blk_mq_complete_request_remote(rq: *mut request) -> bool {
        CORE_FUNC.get_must().sys_blk_mq_complete_request_remote(rq)
    }
    pub(crate) fn sys_blk_mq_rq_from_pdu(pdu: *mut core::ffi::c_void) -> *mut request {
        CORE_FUNC.get_must().sys_blk_mq_rq_from_pdu(pdu)
    }
    pub(crate) fn sys_blk_mq_alloc_tag_set(set: *mut blk_mq_tag_set) -> core::ffi::c_int {
        CORE_FUNC.get_must().sys_blk_mq_alloc_tag_set(set)
    }
    pub(crate) fn sys_blk_mq_free_tag_set(set: *mut blk_mq_tag_set) {
        CORE_FUNC.get_must().sys_blk_mq_free_tag_set(set)
    }

    // mutex
    pub(crate) fn sys__mutex_init(
        ptr: *mut mutex,
        name: *const core::ffi::c_char,
        key: *mut lock_class_key,
    ) {
        CORE_FUNC.get_must().sys__mutex_init(ptr, name, key)
    }
    pub(crate) fn sys_mutex_lock(ptr: *mut mutex) {
        CORE_FUNC.get_must().sys_mutex_lock(ptr)
    }
    pub(crate) fn sys_mutex_unlock(ptr: *mut mutex) {
        CORE_FUNC.get_must().sys_mutex_unlock(ptr)
    }

    pub(crate) fn sys_spin_lock_init(
        ptr: *mut spinlock_t,
        name: *const core::ffi::c_char,
        key: *mut lock_class_key,
    ) {
        CORE_FUNC.get_must().sys_spin_lock_init(ptr, name, key)
    }
    pub(crate) fn sys_spin_lock(ptr: *mut spinlock_t) {
        CORE_FUNC.get_must().sys_spin_lock(ptr)
    }
    pub(crate) fn sys_spin_unlock(ptr: *mut spinlock_t) {
        CORE_FUNC.get_must().sys_spin_unlock(ptr)
    }
    pub(crate) fn sys_spin_lock_irqsave(lock: *mut spinlock_t) -> core::ffi::c_ulong {
        CORE_FUNC.get_must().sys_spin_lock_irqsave(lock)
    }
    pub(crate) fn sys_spin_unlock_irqrestore(lock: *mut spinlock_t, flags: core::ffi::c_ulong) {
        CORE_FUNC.get_must().sys_spin_unlock_irqrestore(lock, flags)
    }

    // tree
    pub(crate) fn sys_init_radix_tree(tree: *mut xarray, gfp_mask: gfp_t) {
        CORE_FUNC.get_must().sys_init_radix_tree(tree, gfp_mask);
    }
    pub(crate) fn sys_radix_tree_insert(
        arg1: *mut xarray,
        index: core::ffi::c_ulong,
        arg2: *mut core::ffi::c_void,
    ) -> core::ffi::c_int {
        CORE_FUNC
            .get_must()
            .sys_radix_tree_insert(arg1, index, arg2)
    }

    pub(crate) fn sys_radix_tree_lookup(
        arg1: *const xarray,
        arg2: core::ffi::c_ulong,
    ) -> *mut core::ffi::c_void {
        CORE_FUNC.get_must().sys_radix_tree_lookup(arg1, arg2)
    }

    pub(crate) fn sys_radix_tree_delete(
        arg1: *mut xarray,
        arg2: core::ffi::c_ulong,
    ) -> *mut core::ffi::c_void {
        CORE_FUNC.get_must().sys_radix_tree_delete(arg1, arg2)
    }

    pub(crate) fn sys_radix_tree_iter_init(
        iter: *mut radix_tree_iter,
        start: core::ffi::c_ulong,
    ) -> *mut *mut core::ffi::c_void {
        CORE_FUNC.get_must().sys_radix_tree_iter_init(iter, start)
    }

    pub(crate) fn sys_radix_tree_next_chunk(
        arg1: *const xarray,
        iter: *mut radix_tree_iter,
        flags: core::ffi::c_uint,
    ) -> *mut *mut core::ffi::c_void {
        CORE_FUNC
            .get_must()
            .sys_radix_tree_next_chunk(arg1, iter, flags)
    }
    pub(crate) fn sys_radix_tree_next_slot(
        slot: *mut *mut core::ffi::c_void,
        iter: *mut radix_tree_iter,
        flags: core::ffi::c_uint,
    ) -> *mut *mut core::ffi::c_void {
        CORE_FUNC
            .get_must()
            .sys_radix_tree_next_slot(slot, iter, flags)
    }

    // time
    pub(crate) fn sys_hrtimer_init(
        timer: *mut hrtimer,
        which_clock: clockid_t,
        mode: hrtimer_mode,
    ) {
        CORE_FUNC
            .get_must()
            .sys_hrtimer_init(timer, which_clock, mode);
    }
    pub(crate) fn sys_hrtimer_cancel(timer: *mut hrtimer) -> core::ffi::c_int {
        CORE_FUNC.get_must().sys_hrtimer_cancel(timer)
    }
    pub(crate) fn sys_hrtimer_start_range_ns(
        timer: *mut hrtimer,
        tim: ktime_t,
        range_ns: u64_,
        mode: hrtimer_mode,
    ) {
        CORE_FUNC
            .get_must()
            .sys_hrtimer_start_range_ns(timer, tim, range_ns, mode);
    }
}

pub use bindings::PAGE_SIZE;
use kbind::blk_mq_tag_set;

impl<T> OnceGet<T> for Once<T> {
    fn get_must(&self) -> &T {
        unsafe { self.get_unchecked() }
    }
}

pub trait OnceGet<T> {
    fn get_must(&self) -> &T;
}
