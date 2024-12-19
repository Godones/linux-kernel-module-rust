#![feature(allocator_api)]
#![feature(try_with_capacity)]
#![feature(associated_type_defaults)]
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
pub mod console;
pub mod domain_info;
pub mod kernel;

pub type LinuxResult<T> = Result<T, LinuxErrno>;
pub type LinuxError = LinuxErrno;

use bindings::*;
pub use kbind::safe_ptr::SafePtr;
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
    fn sys_blk_queue_virt_boundary(&self, arg1: *mut request_queue, arg2: core::ffi::c_ulong);
    fn sys_blk_queue_max_hw_sectors(&self, arg1: *mut request_queue, arg2: core::ffi::c_uint);
    fn sys_blk_queue_max_segments(&self, arg1: *mut request_queue, arg2: core::ffi::c_ushort);
    fn sys_blk_rq_nr_phys_segments(&self, rq: *mut request) -> core::ffi::c_ushort;
    fn sys__blk_rq_map_sg(
        &self,
        q: *mut request_queue,
        rq: *mut request,
        sglist: *mut scatterlist,
        last_sg: *mut *mut scatterlist,
    ) -> core::ffi::c_int;
    fn sys_blk_rq_payload_bytes(&self, rq: *mut request) -> core::ffi::c_uint;
    fn sys_blk_mq_init_queue(&self, arg1: *mut blk_mq_tag_set) -> *mut request_queue;
    fn sys_blk_mq_alloc_request(
        &self,
        q: *mut request_queue,
        opf: blk_opf_t,
        flags: blk_mq_req_flags_t,
    ) -> *mut request;
    fn sys_blk_mq_free_request(&self, rq: *mut request);
    fn sys_blk_execute_rq(&self, rq: *mut request, at_head: bool_) -> blk_status_t;
    fn sys_blk_status_to_errno(&self, status: blk_status_t) -> core::ffi::c_int;
    fn sys_blk_mq_tag_to_rq(&self, tags: *mut blk_mq_tags, tag: core::ffi::c_uint) -> *mut request;
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

    // rcu
    fn sys_rcu_read_lock(&self);
    fn sys_rcu_read_unlock(&self);
    fn sys_synchronize_rcu(&self);

    // device
    fn sys_dev_name(&self, dev: *const device) -> *const core::ffi::c_char;
    fn sys_dma_set_mask(&self, dev: *mut device, mask: u64_) -> core::ffi::c_int;
    fn sys_dma_set_coherent_mask(&self, dev: *mut device, mask: u64_) -> core::ffi::c_int;
    fn sys_dma_map_sg_attrs(
        &self,
        dev: *mut device,
        sg: *mut scatterlist,
        nents: core::ffi::c_int,
        dir: dma_data_direction,
        attrs: core::ffi::c_ulong,
    ) -> core::ffi::c_uint;
    fn sys_dma_unmap_sg_attrs(
        &self,
        dev: *mut device,
        sg: *mut scatterlist,
        nents: core::ffi::c_int,
        dir: dma_data_direction,
        attrs: core::ffi::c_ulong,
    );
    fn sys_get_device(&self, dev: *mut device);
    fn sys_put_device(&self, dev: *mut device);
    fn sys_request_threaded_irq(
        &self,
        irq: core::ffi::c_uint,
        handler: irq_handler_t,
        thread_fn: irq_handler_t,
        flags: core::ffi::c_ulong,
        name: *const core::ffi::c_char,
        dev_id: *mut core::ffi::c_void,
    ) -> core::ffi::c_int;
    fn sys_free_irq(
        &self,
        arg1: core::ffi::c_uint,
        arg2: *mut core::ffi::c_void,
    ) -> *const core::ffi::c_void;

    // DMA
    fn sys_dma_alloc_attrs(
        &self,
        dev: *mut device,
        size: usize,
        dma_handle: *mut dma_addr_t,
        flag: gfp_t,
        attrs: core::ffi::c_ulong,
    ) -> *mut core::ffi::c_void;
    fn sys_dma_free_attrs(
        &self,
        dev: *mut device,
        size: usize,
        cpu_addr: *mut core::ffi::c_void,
        dma_handle: u64,
        attrs: core::ffi::c_ulong,
    );
    fn sys_dma_pool_create(
        &self,
        name: *const core::ffi::c_char,
        dev: *mut device,
        size: usize,
        align: usize,
        boundary: usize,
    ) -> *mut dma_pool;
    fn sys_dma_pool_alloc(
        &self,
        pool: *mut dma_pool,
        flag: gfp_t,
        dma_handle: *mut dma_addr_t,
    ) -> *mut core::ffi::c_void;
    fn sys_dma_pool_free(
        &self,
        pool: *mut dma_pool,
        vaddr: *mut core::ffi::c_void,
        dma_handle: dma_addr_t,
    );
    fn sys_dma_pool_destroy(&self, pool: *mut dma_pool);

    // IO REMAP
    fn sys_ioremap(
        &self,
        offset: resource_size_t,
        size: core::ffi::c_ulong,
    ) -> *mut core::ffi::c_void;
    fn sys_memcpy_fromio(
        &self,
        arg1: *mut core::ffi::c_void,
        arg2: *const core::ffi::c_void,
        arg3: usize,
    );
    fn sys_iounmap(&self, addr: *mut core::ffi::c_void);
    fn sys_readb(&self, addr: *const core::ffi::c_void) -> u8_;
    fn sys_readw(&self, addr: *const core::ffi::c_void) -> u16_;
    fn sys_readl(&self, addr: *const core::ffi::c_void) -> u32_;
    fn sys_readq(&self, addr: *const core::ffi::c_void) -> u64_;

    fn sys_writeb(&self, value: u8_, addr: *mut core::ffi::c_void);
    fn sys_writew(&self, value: u16_, addr: *mut core::ffi::c_void);
    fn sys_writel(&self, value: u32_, addr: *mut core::ffi::c_void);
    fn sys_writeq(&self, value: u64_, addr: *mut core::ffi::c_void);

    fn sys_readb_relaxed(&self, addr: *const core::ffi::c_void) -> u8_;
    fn sys_readw_relaxed(&self, addr: *const core::ffi::c_void) -> u16_;
    fn sys_readl_relaxed(&self, addr: *const core::ffi::c_void) -> u32_;
    fn sys_readq_relaxed(&self, addr: *const core::ffi::c_void) -> u64_;

    fn sys_writeb_relaxed(&self, value: u8_, addr: *mut core::ffi::c_void);
    fn sys_writew_relaxed(&self, value: u16_, addr: *mut core::ffi::c_void);
    fn sys_writel_relaxed(&self, value: u32_, addr: *mut core::ffi::c_void);
    fn sys_writeq_relaxed(&self, value: u64_, addr: *mut core::ffi::c_void);

    //PCI
    #[must_use]
    fn sys__pci_register_driver(
        &self,
        arg1: *mut pci_driver,
        arg2: *mut module,
        mod_name: *const core::ffi::c_char,
    ) -> core::ffi::c_int;
    fn sys_pci_unregister_driver(&self, arg1: *mut pci_driver);
    fn sys_pci_set_drvdata(&self, pdev: *mut pci_dev, data: *mut core::ffi::c_void);
    fn sys_pci_get_drvdata(&self, pdev: *mut pci_dev) -> *mut core::ffi::c_void;
    fn sys_pci_enable_device_mem(&self, pdev: *mut pci_dev) -> core::ffi::c_int;
    fn sys_pci_set_master(&self, pdev: *mut pci_dev);
    fn sys_pci_select_bars(&self, dev: *mut pci_dev, flags: core::ffi::c_ulong)
        -> core::ffi::c_int;
    fn sys_pci_request_selected_regions(
        &self,
        arg1: *mut pci_dev,
        arg2: core::ffi::c_int,
        arg3: *const core::ffi::c_char,
    ) -> core::ffi::c_int;
    fn sys_pci_alloc_irq_vectors_affinity(
        &self,
        dev: *mut pci_dev,
        min_vecs: core::ffi::c_uint,
        max_vecs: core::ffi::c_uint,
        flags: core::ffi::c_uint,
        affd: *mut irq_affinity,
    ) -> core::ffi::c_int;
    fn sys_pci_free_irq_vectors(&self, pdev: *mut pci_dev);
    fn sys_pci_irq_vector(&self, pdev: *mut pci_dev, nr: core::ffi::c_uint) -> core::ffi::c_int;
    fn sys_blk_mq_pci_map_queues(
        &self,
        qmap: *mut blk_mq_queue_map,
        pdev: *mut pci_dev,
        offset: core::ffi::c_int,
    );
    fn sys_blk_mq_map_queues(&self, qmap: *mut blk_mq_queue_map);
    fn sys_dma_map_page_attrs(
        &self,
        dev: *mut device,
        page: *mut page,
        offset: usize,
        size: usize,
        dir: dma_data_direction,
        attrs: core::ffi::c_ulong,
    ) -> dma_addr_t;
    fn sys_dma_unmap_page_attrs(
        &self,
        dev: *mut device,
        handle: dma_addr_t,
        size: usize,
        dir: dma_data_direction,
        attrs: core::ffi::c_ulong,
    );

    fn sys_num_possible_cpus(&self) -> core::ffi::c_uint;
    fn sys_mdelay(&self, msecs: u64);
    fn sys_ktime_get_ns(&self) -> u64;
    fn sys_sg_next(&self, sg: *const scatterlist) -> *const scatterlist;
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

    // blk
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
    #[allow(unused)]
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
    pub(crate) fn sys_blk_queue_virt_boundary(arg1: *mut request_queue, arg2: core::ffi::c_ulong) {
        CORE_FUNC.get_must().sys_blk_queue_virt_boundary(arg1, arg2)
    }
    pub(crate) fn sys_blk_queue_max_hw_sectors(arg1: *mut request_queue, arg2: core::ffi::c_uint) {
        CORE_FUNC
            .get_must()
            .sys_blk_queue_max_hw_sectors(arg1, arg2)
    }
    pub(crate) fn sys_blk_queue_max_segments(arg1: *mut request_queue, arg2: core::ffi::c_ushort) {
        CORE_FUNC.get_must().sys_blk_queue_max_segments(arg1, arg2)
    }

    pub(crate) fn sys_blk_rq_nr_phys_segments(rq: *mut request) -> core::ffi::c_ushort {
        CORE_FUNC.get_must().sys_blk_rq_nr_phys_segments(rq)
    }

    pub(crate) fn sys__blk_rq_map_sg(
        q: *mut request_queue,
        rq: *mut request,
        sglist: *mut scatterlist,
        last_sg: *mut *mut scatterlist,
    ) -> core::ffi::c_int {
        CORE_FUNC
            .get_must()
            .sys__blk_rq_map_sg(q, rq, sglist, last_sg)
    }
    pub(crate) fn sys_blk_rq_payload_bytes(rq: *mut request) -> core::ffi::c_uint {
        CORE_FUNC.get_must().sys_blk_rq_payload_bytes(rq)
    }
    pub(crate) fn sys_blk_mq_init_queue(arg1: *mut blk_mq_tag_set) -> *mut request_queue {
        CORE_FUNC.get_must().sys_blk_mq_init_queue(arg1)
    }
    pub(crate) fn sys_blk_mq_alloc_request(
        q: *mut request_queue,
        opf: blk_opf_t,
        flags: blk_mq_req_flags_t,
    ) -> *mut request {
        CORE_FUNC.get_must().sys_blk_mq_alloc_request(q, opf, flags)
    }
    pub(crate) fn sys_blk_execute_rq(rq: *mut request, at_head: bool_) -> blk_status_t {
        CORE_FUNC.get_must().sys_blk_execute_rq(rq, at_head)
    }
    pub(crate) fn sys_blk_status_to_errno(status: blk_status_t) -> core::ffi::c_int {
        CORE_FUNC.get_must().sys_blk_status_to_errno(status)
    }
    pub(crate) fn sys_blk_mq_free_request(rq: *mut request) {
        CORE_FUNC.get_must().sys_blk_mq_free_request(rq)
    }
    pub(crate) fn sys_blk_mq_tag_to_rq(
        tags: *mut blk_mq_tags,
        tag: core::ffi::c_uint,
    ) -> *mut request {
        CORE_FUNC.get_must().sys_blk_mq_tag_to_rq(tags, tag)
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

    // rcu
    pub(crate) fn sys_rcu_read_lock() {
        CORE_FUNC.get_must().sys_rcu_read_lock();
    }
    pub(crate) fn sys_rcu_read_unlock() {
        CORE_FUNC.get_must().sys_rcu_read_unlock();
    }
    pub(crate) fn sys_synchronize_rcu() {
        CORE_FUNC.get_must().sys_synchronize_rcu();
    }

    // device
    pub(crate) fn sys_dev_name(dev: *const device) -> *const core::ffi::c_char {
        CORE_FUNC.get_must().sys_dev_name(dev)
    }
    pub(crate) fn sys_dma_set_mask(dev: *mut device, mask: u64_) -> core::ffi::c_int {
        CORE_FUNC.get_must().sys_dma_set_mask(dev, mask)
    }
    pub(crate) fn sys_dma_set_coherent_mask(dev: *mut device, mask: u64_) -> core::ffi::c_int {
        CORE_FUNC.get_must().sys_dma_set_coherent_mask(dev, mask)
    }
    pub(crate) fn sys_dma_map_sg_attrs(
        dev: *mut device,
        sg: *mut scatterlist,
        nents: core::ffi::c_int,
        dir: dma_data_direction,
        attrs: core::ffi::c_ulong,
    ) -> core::ffi::c_uint {
        CORE_FUNC
            .get_must()
            .sys_dma_map_sg_attrs(dev, sg, nents, dir, attrs)
    }

    pub(crate) fn sys_dma_unmap_sg_attrs(
        dev: *mut device,
        sg: *mut scatterlist,
        nents: core::ffi::c_int,
        dir: dma_data_direction,
        attrs: core::ffi::c_ulong,
    ) {
        CORE_FUNC
            .get_must()
            .sys_dma_unmap_sg_attrs(dev, sg, nents, dir, attrs)
    }
    pub(crate) fn sys_get_device(dev: *mut device) {
        CORE_FUNC.get_must().sys_get_device(dev)
    }
    pub(crate) fn sys_put_device(dev: *mut device) {
        CORE_FUNC.get_must().sys_put_device(dev)
    }

    #[must_use]
    pub(crate) fn sys_request_threaded_irq(
        irq: core::ffi::c_uint,
        handler: irq_handler_t,
        thread_fn: irq_handler_t,
        flags: core::ffi::c_ulong,
        name: *const core::ffi::c_char,
        dev: *mut core::ffi::c_void,
    ) -> core::ffi::c_int {
        CORE_FUNC
            .get_must()
            .sys_request_threaded_irq(irq, handler, thread_fn, flags, name, dev)
    }
    pub(crate) fn sys_free_irq(
        arg1: core::ffi::c_uint,
        arg2: *mut core::ffi::c_void,
    ) -> *const core::ffi::c_void {
        CORE_FUNC.get_must().sys_free_irq(arg1, arg2)
    }

    // dma
    pub(crate) fn sys_dma_free_attrs(
        dev: *mut device,
        size: usize,
        cpu_addr: *mut core::ffi::c_void,
        dma_handle: dma_addr_t,
        attrs: core::ffi::c_ulong,
    ) {
        CORE_FUNC
            .get_must()
            .sys_dma_free_attrs(dev, size, cpu_addr, dma_handle, attrs)
    }
    pub(crate) fn sys_dma_alloc_attrs(
        dev: *mut device,
        size: usize,
        dma_handle: *mut dma_addr_t,
        flag: gfp_t,
        attrs: core::ffi::c_ulong,
    ) -> *mut core::ffi::c_void {
        CORE_FUNC
            .get_must()
            .sys_dma_alloc_attrs(dev, size, dma_handle, flag, attrs)
    }
    pub(crate) fn sys_dma_pool_create(
        name: *const core::ffi::c_char,
        dev: *mut device,
        size: usize,
        align: usize,
        allocation: usize,
    ) -> *mut dma_pool {
        CORE_FUNC
            .get_must()
            .sys_dma_pool_create(name, dev, size, align, allocation)
    }
    pub(crate) fn sys_dma_pool_alloc(
        pool: *mut dma_pool,
        mem_flags: gfp_t,
        handle: *mut dma_addr_t,
    ) -> *mut core::ffi::c_void {
        CORE_FUNC
            .get_must()
            .sys_dma_pool_alloc(pool, mem_flags, handle)
    }
    pub(crate) fn sys_dma_pool_free(
        pool: *mut dma_pool,
        vaddr: *mut core::ffi::c_void,
        addr: dma_addr_t,
    ) {
        CORE_FUNC.get_must().sys_dma_pool_free(pool, vaddr, addr)
    }
    pub(crate) fn sys_dma_pool_destroy(pool: *mut dma_pool) {
        CORE_FUNC.get_must().sys_dma_pool_destroy(pool)
    }

    // IO REMAP
    pub(crate) fn sys_ioremap(
        offset: resource_size_t,
        size: core::ffi::c_ulong,
    ) -> *mut core::ffi::c_void {
        CORE_FUNC.get_must().sys_ioremap(offset, size)
    }
    pub(crate) fn sys_memcpy_fromio(
        arg1: *mut core::ffi::c_void,
        arg2: *const core::ffi::c_void,
        arg3: usize,
    ) {
        CORE_FUNC.get_must().sys_memcpy_fromio(arg1, arg2, arg3)
    }
    pub(crate) fn sys_iounmap(addr: *mut core::ffi::c_void) {
        CORE_FUNC.get_must().sys_iounmap(addr)
    }

    pub mod io {
        use kbind::*;

        use super::CORE_FUNC;
        use crate::OnceGet;

        pub(crate) fn readb(addr: *const core::ffi::c_void) -> u8_ {
            CORE_FUNC.get_must().sys_readb(addr)
        }
        pub(crate) fn readw(addr: *const core::ffi::c_void) -> u16_ {
            CORE_FUNC.get_must().sys_readw(addr)
        }
        pub(crate) fn readl(addr: *const core::ffi::c_void) -> u32_ {
            CORE_FUNC.get_must().sys_readl(addr)
        }
        pub(crate) fn readq(addr: *const core::ffi::c_void) -> u64_ {
            CORE_FUNC.get_must().sys_readq(addr)
        }

        pub(crate) fn writeb(value: u8_, addr: *mut core::ffi::c_void) {
            CORE_FUNC.get_must().sys_writeb(value, addr)
        }
        pub(crate) fn writew(value: u16_, addr: *mut core::ffi::c_void) {
            CORE_FUNC.get_must().sys_writew(value, addr)
        }
        pub(crate) fn writel(value: u32_, addr: *mut core::ffi::c_void) {
            CORE_FUNC.get_must().sys_writel(value, addr)
        }
        pub(crate) fn writeq(value: u64_, addr: *mut core::ffi::c_void) {
            CORE_FUNC.get_must().sys_writeq(value, addr)
        }

        pub(crate) fn readb_relaxed(addr: *const core::ffi::c_void) -> u8_ {
            CORE_FUNC.get_must().sys_readb_relaxed(addr)
        }
        pub(crate) fn readw_relaxed(addr: *const core::ffi::c_void) -> u16_ {
            CORE_FUNC.get_must().sys_readw_relaxed(addr)
        }
        pub(crate) fn readl_relaxed(addr: *const core::ffi::c_void) -> u32_ {
            CORE_FUNC.get_must().sys_readl_relaxed(addr)
        }
        pub(crate) fn readq_relaxed(addr: *const core::ffi::c_void) -> u64_ {
            CORE_FUNC.get_must().sys_readq_relaxed(addr)
        }

        pub(crate) fn writeb_relaxed(value: u8_, addr: *mut core::ffi::c_void) {
            CORE_FUNC.get_must().sys_writeb_relaxed(value, addr)
        }
        pub(crate) fn writew_relaxed(value: u16_, addr: *mut core::ffi::c_void) {
            CORE_FUNC.get_must().sys_writew_relaxed(value, addr)
        }
        pub(crate) fn writel_relaxed(value: u32_, addr: *mut core::ffi::c_void) {
            CORE_FUNC.get_must().sys_writel_relaxed(value, addr)
        }
        pub(crate) fn writeq_relaxed(value: u64_, addr: *mut core::ffi::c_void) {
            CORE_FUNC.get_must().sys_writeq_relaxed(value, addr)
        }
    }

    // PCI
    #[must_use]
    pub(crate) fn sys__pci_register_driver(
        arg1: *mut pci_driver,
        arg2: *mut module,
        mod_name: *const core::ffi::c_char,
    ) -> core::ffi::c_int {
        CORE_FUNC
            .get_must()
            .sys__pci_register_driver(arg1, arg2, mod_name)
    }

    pub(crate) fn sys_pci_unregister_driver(dev: *mut pci_driver) {
        CORE_FUNC.get_must().sys_pci_unregister_driver(dev)
    }
    pub(crate) fn sys_pci_set_drvdata(pdev: *mut pci_dev, data: *mut core::ffi::c_void) {
        CORE_FUNC.get_must().sys_pci_set_drvdata(pdev, data)
    }
    pub(crate) fn sys_pci_get_drvdata(pdev: *mut pci_dev) -> *mut core::ffi::c_void {
        CORE_FUNC.get_must().sys_pci_get_drvdata(pdev)
    }
    #[must_use]
    pub(crate) fn sys_pci_enable_device_mem(dev: *mut pci_dev) -> core::ffi::c_int {
        CORE_FUNC.get_must().sys_pci_enable_device_mem(dev)
    }
    pub(crate) fn sys_pci_set_master(dev: *mut pci_dev) {
        CORE_FUNC.get_must().sys_pci_set_master(dev)
    }
    pub(crate) fn sys_pci_select_bars(
        dev: *mut pci_dev,
        flags: core::ffi::c_ulong,
    ) -> core::ffi::c_int {
        CORE_FUNC.get_must().sys_pci_select_bars(dev, flags)
    }
    pub(crate) fn sys_pci_request_selected_regions(
        arg1: *mut pci_dev,
        arg2: core::ffi::c_int,
        arg3: *const core::ffi::c_char,
    ) -> core::ffi::c_int {
        CORE_FUNC
            .get_must()
            .sys_pci_request_selected_regions(arg1, arg2, arg3)
    }

    pub(crate) fn sys_pci_alloc_irq_vectors_affinity(
        dev: *mut pci_dev,
        min_vecs: core::ffi::c_uint,
        max_vecs: core::ffi::c_uint,
        flags: core::ffi::c_uint,
        affd: *mut irq_affinity,
    ) -> core::ffi::c_int {
        CORE_FUNC
            .get_must()
            .sys_pci_alloc_irq_vectors_affinity(dev, min_vecs, max_vecs, flags, affd)
    }
    pub(crate) fn sys_pci_free_irq_vectors(dev: *mut pci_dev) {
        CORE_FUNC.get_must().sys_pci_free_irq_vectors(dev)
    }
    pub(crate) fn sys_pci_irq_vector(dev: *mut pci_dev, nr: core::ffi::c_uint) -> core::ffi::c_int {
        CORE_FUNC.get_must().sys_pci_irq_vector(dev, nr)
    }
    pub fn sys_blk_mq_pci_map_queues(
        qmap: *mut blk_mq_queue_map,
        pdev: *mut pci_dev,
        offset: core::ffi::c_int,
    ) {
        CORE_FUNC
            .get_must()
            .sys_blk_mq_pci_map_queues(qmap, pdev, offset)
    }
    pub fn sys_blk_mq_map_queues(qmap: *mut blk_mq_queue_map) {
        CORE_FUNC.get_must().sys_blk_mq_map_queues(qmap)
    }
    pub fn sys_dma_map_page_attrs(
        dev: *mut device,
        page: *mut page,
        offset: usize,
        size: usize,
        dir: dma_data_direction,
        attrs: core::ffi::c_ulong,
    ) -> dma_addr_t {
        CORE_FUNC
            .get_must()
            .sys_dma_map_page_attrs(dev, page, offset, size, dir, attrs)
    }
    pub fn sys_dma_unmap_page_attrs(
        dev: *mut device,
        addr: dma_addr_t,
        size: usize,
        dir: dma_data_direction,
        attrs: core::ffi::c_ulong,
    ) {
        CORE_FUNC
            .get_must()
            .sys_dma_unmap_page_attrs(dev, addr, size, dir, attrs)
    }

    pub fn sys_num_possible_cpus() -> core::ffi::c_uint {
        CORE_FUNC.get_must().sys_num_possible_cpus()
    }
    pub fn sys_mdelay(ms: u64) {
        CORE_FUNC.get_must().sys_mdelay(ms)
    }
    pub fn sys_ktime_get_ns() -> u64 {
        CORE_FUNC.get_must().sys_ktime_get_ns()
    }
    pub fn sys_sg_next(sg: *const scatterlist) -> *const scatterlist {
        CORE_FUNC.get_must().sys_sg_next(sg)
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
