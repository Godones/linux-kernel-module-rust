use alloc::{string::ToString, sync::Arc};
use core::{
    any::Any,
    ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_ushort, c_void},
    sync::atomic::AtomicBool,
};

use corelib::{domain_info::DomainDataInfo, CoreFunction, LinuxError, LinuxResult};
use interface::*;
use kernel::bindings::*;

use crate::{
    config::FRAME_BITS,
    domain_helper::{resource::DOMAIN_RESOURCE, DOMAIN_CREATE, DOMAIN_INFO},
    domain_loader::creator,
    domain_proxy::{
        block_device::BlockDeviceDomainProxy, empty_device::EmptyDeviceDomainProxy,
        logger::LogDomainProxy,
    },
};

pub static DOMAIN_SYS: &'static dyn CoreFunction = &DomainSyscall;

pub struct DomainSyscall;

impl CoreFunction for DomainSyscall {
    fn sys_alloc_pages(&self, domain_id: u64, n: usize) -> *mut u8 {
        let n = n.next_power_of_two();
        let page = crate::mem::alloc_frames(n);
        // info!(
        //     "[Domain: {}] alloc pages: {}, range:[{:#x}-{:#x}]",
        //     domain_id,
        //     n,
        //     page as usize,
        //     page as usize + n * FRAME_SIZE
        // );
        DOMAIN_RESOURCE
            .lock()
            .insert_page_map(domain_id, (page as usize >> FRAME_BITS, n));
        page
    }

    fn sys_free_pages(&self, domain_id: u64, p: *mut u8, n: usize) {
        let n = n.next_power_of_two();
        debug!("[Domain: {}] free pages: {}, ptr: {:p}", domain_id, n, p);
        DOMAIN_RESOURCE
            .lock()
            .free_page_map(domain_id, p as usize >> FRAME_BITS);
        crate::mem::free_frames(p, n);
    }

    fn sys_write_console(&self, s: &str) {
        print_raw!("{}", s);
    }

    fn sys_backtrace(&self, domain_id: u64) {
        let mut info = DOMAIN_INFO.lock();
        info.domain_list
            .get_mut(&domain_id)
            .map(|d| d.panic_count += 1);
        unwind();
    }

    fn blk_crash_trick(&self) -> bool {
        BLK_CRASH.load(core::sync::atomic::Ordering::Relaxed)
    }

    fn sys_get_domain(&self, name: &str) -> Option<DomainType> {
        super::query_domain(name)
    }

    fn sys_create_domain(
        &self,
        domain_file_name: &str,
        identifier: &mut [u8],
    ) -> LinuxResult<DomainType> {
        DOMAIN_CREATE
            .get()
            .unwrap()
            .create_domain(domain_file_name, identifier)
    }

    fn sys_register_domain(&self, ident: &str, ty: DomainTypeRaw, data: &[u8]) -> LinuxResult<()> {
        creator::register_domain_elf(ident, data.to_vec(), ty);
        Ok(())
    }

    fn sys_update_domain(
        &self,
        old_domain_name: &str,
        new_domain_name: &str,
        ty: DomainTypeRaw,
    ) -> LinuxResult<()> {
        let old_domain = super::query_domain(old_domain_name);
        let old_domain_id = old_domain.as_ref().map(|d| d.domain_id());
        let (domain_info, new_domain_id) = match old_domain {
            Some(DomainType::LogDomain(logger)) => {
                let old_domain_id = logger.domain_id();
                let (id, new_domain, loader) = creator::create_domain_or_empty::<LogDomainProxy, _>(
                    ty,
                    new_domain_name,
                    None,
                    Some(old_domain_id),
                );
                let logger_proxy = logger.downcast_arc::<LogDomainProxy>().unwrap();
                let domain_info = loader.domain_file_info();
                logger_proxy.replace(new_domain, loader)?;
                println!(
                    "Try to replace logger domain {} with {} ok",
                    old_domain_name, new_domain_name
                );
                Ok((domain_info, id))
            }
            Some(DomainType::EmptyDeviceDomain(empty_device)) => {
                let old_domain_id = empty_device.domain_id();
                let (id, new_domain, loader) = creator::create_domain_or_empty::<
                    EmptyDeviceDomainProxy,
                    _,
                >(
                    ty, new_domain_name, None, Some(old_domain_id)
                );
                let empty_device = empty_device
                    .downcast_arc::<EmptyDeviceDomainProxy>()
                    .unwrap();
                let domain_info = loader.domain_file_info();
                empty_device.replace(new_domain, loader)?;
                println!(
                    "Try to replace empty device domain {} with {} ok",
                    old_domain_name, new_domain_name
                );
                Ok((domain_info, id))
            }
            Some(DomainType::BlockDeviceDomain(block_device)) => {
                let old_domain_id = block_device.domain_id();
                let (id, new_domain, loader) = creator::create_domain_or_empty::<
                    BlockDeviceDomainProxy,
                    _,
                >(
                    ty, new_domain_name, None, Some(old_domain_id)
                );
                let block_device = block_device
                    .downcast_arc::<BlockDeviceDomainProxy>()
                    .unwrap();
                let domain_info = loader.domain_file_info();
                block_device.replace(new_domain, loader)?;
                println!(
                    "Try to replace block device domain {} with {} ok",
                    old_domain_name, new_domain_name
                );
                Ok((domain_info, id))
            }
            Some(DomainType::NvmeBlockDeviceDomain(_nvme)) => {
                unimplemented!("replace nvme domain");
            }
            None => {
                println!(
                    "<sys_update_domain> old domain {:?} not found",
                    old_domain_name
                );
                Err(LinuxError::EINVAL)
            }
        }?;
        let domain_data = DomainDataInfo {
            name: old_domain_name.to_string(),
            ty,
            panic_count: 0,
            file_info: domain_info,
        };

        let mut info = DOMAIN_INFO.lock();
        info.domain_list.remove(&old_domain_id.unwrap());
        info.domain_list.insert(new_domain_id, domain_data);
        Ok(())
    }
    fn sys_reload_domain(&self, domain_name: &str) -> LinuxResult<()> {
        let domain = super::query_domain(domain_name).ok_or(LinuxError::EINVAL)?;
        match domain {
            // todo!(release old domain's resource)
            ty => {
                panic!("reload domain {:?} not support", ty);
            }
        }
    }

    fn checkout_shared_data(&self) -> LinuxResult<()> {
        crate::domain_helper::checkout_shared_data();
        Ok(())
    }

    fn domain_info(&self) -> LinuxResult<Arc<dyn Any + Send + Sync>> {
        let info = DOMAIN_INFO.clone();
        Ok(info)
    }

    fn sys_err_ptr(&self, err: c_long) -> *mut c_void {
        unsafe { kernel::bindings::ERR_PTR(err) }
    }

    fn sys_is_err(&self, ptr: *const c_void) -> bool {
        unsafe { kernel::bindings::is_err(ptr) }
    }

    fn sys_ptr_err(&self, ptr: *const c_void) -> c_long {
        unsafe { kernel::bindings::ptr_err(ptr) }
    }

    fn sys_errno_to_blk_status(&self, errno: c_int) -> blk_status_t {
        unsafe { kernel::bindings::errno_to_blk_status(errno) }
    }

    fn sys_bio_advance_iter_single(&self, bio: *const bio, iter: *mut bvec_iter, bytes: c_uint) {
        unsafe { kernel::bindings::bio_advance_iter_single(bio, iter, bytes) }
    }

    fn sys_kmap(&self, page: *mut page) -> *mut c_void {
        unsafe { kernel::bindings::kmap(page) }
    }

    fn sys_kunmap(&self, page: *mut page) {
        unsafe { kernel::bindings::kunmap(page) }
    }

    fn sys_kmap_atomic(&self, page: *mut page) -> *mut c_void {
        unsafe { kernel::bindings::kmap_atomic(page) }
    }

    fn sys_kunmap_atomic(&self, address: *mut c_void) {
        unsafe { kernel::bindings::kunmap_atomic(address) }
    }

    fn sys__alloc_pages(&self, gfp: gfp_t, order: c_uint) -> *mut page {
        unsafe { kernel::bindings::alloc_pages(gfp, order) }
    }

    fn sys__free_pages(&self, page: *mut page, order: c_uint) {
        unsafe { kernel::bindings::__free_pages(page, order) }
    }

    fn sys__blk_mq_alloc_disk(
        &self,
        set: *mut blk_mq_tag_set,
        queuedata: *mut c_void,
        lkclass: *mut lock_class_key,
    ) -> *mut gendisk {
        unsafe { kernel::bindings::__blk_mq_alloc_disk(set, queuedata, lkclass) }
    }

    fn sys_device_add_disk(
        &self,
        parent: *mut device,
        disk: *mut gendisk,
        groups: *mut *const attribute_group,
    ) -> c_int {
        unsafe { kernel::bindings::device_add_disk(parent, disk, groups) }
    }

    fn sys_set_capacity(&self, disk: *mut gendisk, size: sector_t) {
        unsafe { kernel::bindings::set_capacity(disk, size) }
    }

    fn sys_blk_queue_logical_block_size(&self, arg1: *mut request_queue, arg2: c_uint) {
        unsafe { kernel::bindings::blk_queue_logical_block_size(arg1, arg2) }
    }

    fn sys_blk_queue_physical_block_size(&self, arg1: *mut request_queue, arg2: c_uint) {
        unsafe { kernel::bindings::blk_queue_physical_block_size(arg1, arg2) }
    }

    fn sys_blk_queue_flag_set(&self, flag: c_uint, q: *mut request_queue) {
        unsafe { kernel::bindings::blk_queue_flag_set(flag, q) }
    }

    fn sys_blk_queue_flag_clear(&self, flag: c_uint, q: *mut request_queue) {
        unsafe { kernel::bindings::blk_queue_flag_clear(flag, q) }
    }

    fn sys_del_gendisk(&self, disk: *mut gendisk) {
        unsafe { kernel::bindings::del_gendisk(disk) }
    }

    fn sys_blk_mq_rq_to_pdu(&self, rq: *mut request) -> *mut c_void {
        unsafe { kernel::bindings::blk_mq_rq_to_pdu(rq) }
    }

    fn sys_blk_mq_start_request(&self, rq: *mut request) {
        unsafe { kernel::bindings::blk_mq_start_request(rq) }
    }

    fn sys_blk_mq_end_request(&self, rq: *mut request, status: blk_status_t) {
        unsafe { kernel::bindings::blk_mq_end_request(rq, status) }
    }

    fn sys_blk_mq_complete_request_remote(&self, rq: *mut request) -> bool {
        unsafe { kernel::bindings::blk_mq_complete_request_remote(rq) }
    }

    fn sys_blk_mq_rq_from_pdu(&self, pdu: *mut c_void) -> *mut request {
        unsafe { kernel::bindings::blk_mq_rq_from_pdu(pdu) }
    }

    fn sys_blk_mq_alloc_tag_set(&self, set: *mut blk_mq_tag_set) -> c_int {
        unsafe { kernel::bindings::blk_mq_alloc_tag_set(set) }
    }

    fn sys_blk_mq_free_tag_set(&self, set: *mut blk_mq_tag_set) {
        unsafe { kernel::bindings::blk_mq_free_tag_set(set) }
    }

    fn sys_blk_queue_virt_boundary(&self, arg1: *mut request_queue, arg2: c_ulong) {
        unsafe { kernel::bindings::blk_queue_virt_boundary(arg1, arg2) }
    }

    fn sys_blk_queue_max_hw_sectors(&self, arg1: *mut request_queue, arg2: c_uint) {
        unsafe { kernel::bindings::blk_queue_max_hw_sectors(arg1, arg2) }
    }

    fn sys_blk_queue_max_segments(&self, arg1: *mut request_queue, arg2: c_ushort) {
        unsafe { kernel::bindings::blk_queue_max_segments(arg1, arg2) }
    }

    fn sys_blk_rq_nr_phys_segments(&self, rq: *mut request) -> c_ushort {
        unsafe { kernel::bindings::blk_rq_nr_phys_segments(rq) }
    }

    fn sys__blk_rq_map_sg(
        &self,
        q: *mut request_queue,
        rq: *mut request,
        sglist: *mut scatterlist,
        last_sg: *mut *mut scatterlist,
    ) -> c_int {
        unsafe { kernel::bindings::__blk_rq_map_sg(q, rq, sglist, last_sg) }
    }

    fn sys_blk_rq_payload_bytes(&self, rq: *mut request) -> c_uint {
        unsafe { kernel::bindings::blk_rq_payload_bytes(rq) }
    }

    fn sys_blk_mq_init_queue(&self, arg1: *mut blk_mq_tag_set) -> *mut request_queue {
        unsafe { kernel::bindings::blk_mq_init_queue(arg1) }
    }

    fn sys_blk_mq_alloc_request(
        &self,
        q: *mut request_queue,
        opf: blk_opf_t,
        flags: blk_mq_req_flags_t,
    ) -> *mut request {
        unsafe { kernel::bindings::blk_mq_alloc_request(q, opf, flags) }
    }

    fn sys_blk_mq_free_request(&self, rq: *mut request) {
        unsafe { kernel::bindings::blk_mq_free_request(rq) }
    }

    fn sys_blk_execute_rq(&self, rq: *mut request, at_head: bool_) -> blk_status_t {
        unsafe { kernel::bindings::blk_execute_rq(rq, at_head) }
    }

    fn sys_blk_status_to_errno(&self, status: blk_status_t) -> c_int {
        unsafe { kernel::bindings::blk_status_to_errno(status) }
    }

    fn sys_blk_mq_tag_to_rq(&self, tags: *mut blk_mq_tags, tag: c_uint) -> *mut request {
        unsafe { kernel::bindings::blk_mq_tag_to_rq(tags, tag) }
    }

    fn sys__mutex_init(&self, ptr: *mut mutex, name: *const c_char, key: *mut lock_class_key) {
        unsafe { kernel::bindings::__mutex_init(ptr, name, key) }
    }

    fn sys_mutex_lock(&self, ptr: *mut mutex) {
        unsafe { kernel::bindings::mutex_lock(ptr) }
    }

    fn sys_mutex_unlock(&self, ptr: *mut mutex) {
        unsafe { kernel::bindings::mutex_unlock(ptr) }
    }

    fn sys_spin_lock_init(
        &self,
        ptr: *mut spinlock_t,
        name: *const c_char,
        key: *mut lock_class_key,
    ) {
        unsafe { kernel::bindings::spin_lock_init(ptr, name, key) }
    }

    fn sys_spin_lock(&self, ptr: *mut spinlock_t) {
        unsafe { kernel::bindings::spin_lock(ptr) }
    }

    fn sys_spin_unlock(&self, ptr: *mut spinlock_t) {
        unsafe { kernel::bindings::spin_unlock(ptr) }
    }

    fn sys_spin_lock_irqsave(&self, lock: *mut spinlock_t) -> c_ulong {
        unsafe { kernel::bindings::spin_lock_irqsave(lock) }
    }

    fn sys_spin_unlock_irqrestore(&self, lock: *mut spinlock_t, flags: c_ulong) {
        unsafe { kernel::bindings::spin_unlock_irqrestore(lock, flags) }
    }

    fn sys_init_radix_tree(&self, tree: *mut xarray, gfp_mask: gfp_t) {
        unsafe { kernel::bindings::init_radix_tree(tree, gfp_mask) }
    }

    fn sys_radix_tree_insert(&self, arg1: *mut xarray, index: c_ulong, arg2: *mut c_void) -> c_int {
        unsafe { kernel::bindings::radix_tree_insert(arg1, index, arg2) }
    }

    fn sys_radix_tree_lookup(&self, arg1: *const xarray, arg2: c_ulong) -> *mut c_void {
        unsafe { kernel::bindings::radix_tree_lookup(arg1, arg2) }
    }

    fn sys_radix_tree_delete(&self, arg1: *mut xarray, arg2: c_ulong) -> *mut c_void {
        unsafe { kernel::bindings::radix_tree_delete(arg1, arg2) }
    }

    fn sys_radix_tree_iter_init(
        &self,
        iter: *mut radix_tree_iter,
        start: c_ulong,
    ) -> *mut *mut c_void {
        unsafe { kernel::bindings::radix_tree_iter_init(iter, start) }
    }

    fn sys_radix_tree_next_chunk(
        &self,
        arg1: *const xarray,
        iter: *mut radix_tree_iter,
        flags: c_uint,
    ) -> *mut *mut c_void {
        unsafe { kernel::bindings::radix_tree_next_chunk(arg1, iter, flags) }
    }

    fn sys_radix_tree_next_slot(
        &self,
        slot: *mut *mut c_void,
        iter: *mut radix_tree_iter,
        flags: c_uint,
    ) -> *mut *mut c_void {
        unsafe { kernel::bindings::radix_tree_next_slot(slot, iter, flags) }
    }

    fn sys_hrtimer_init(&self, timer: *mut hrtimer, which_clock: clockid_t, mode: hrtimer_mode) {
        unsafe { kernel::bindings::hrtimer_init(timer, which_clock, mode) }
    }

    fn sys_hrtimer_cancel(&self, timer: *mut hrtimer) -> c_int {
        unsafe { kernel::bindings::hrtimer_cancel(timer) }
    }

    fn sys_hrtimer_start_range_ns(
        &self,
        timer: *mut hrtimer,
        tim: ktime_t,
        range_ns: u64_,
        mode: hrtimer_mode,
    ) {
        unsafe { kernel::bindings::hrtimer_start_range_ns(timer, tim, range_ns, mode) }
    }

    fn sys_rcu_read_lock(&self) {
        unsafe { kernel::bindings::rust_helper_rcu_read_unlock() }
    }

    fn sys_rcu_read_unlock(&self) {
        unsafe { kernel::bindings::rust_helper_rcu_read_unlock() }
    }

    fn sys_synchronize_rcu(&self) {
        unsafe { kernel::bindings::rust_helper_synchronize_rcu() }
    }

    fn sys_dev_name(&self, dev: *const device) -> *const c_char {
        unsafe { kernel::bindings::dev_name(dev) }
    }

    fn sys_dma_set_mask(&self, dev: *mut device, mask: u64_) -> c_int {
        unsafe { kernel::bindings::dma_set_mask(dev, mask) }
    }

    fn sys_dma_set_coherent_mask(&self, dev: *mut device, mask: u64_) -> c_int {
        unsafe { kernel::bindings::dma_set_coherent_mask(dev, mask) }
    }

    fn sys_dma_map_sg_attrs(
        &self,
        dev: *mut device,
        sg: *mut scatterlist,
        nents: c_int,
        dir: dma_data_direction,
        attrs: c_ulong,
    ) -> c_uint {
        unsafe { kernel::bindings::dma_map_sg_attrs(dev, sg, nents, dir, attrs) }
    }

    fn sys_dma_unmap_sg_attrs(
        &self,
        dev: *mut device,
        sg: *mut scatterlist,
        nents: c_int,
        dir: dma_data_direction,
        attrs: c_ulong,
    ) {
        unsafe { kernel::bindings::dma_unmap_sg_attrs(dev, sg, nents, dir, attrs) }
    }

    fn sys_get_device(&self, dev: *mut device) {
        unsafe {
            kernel::bindings::get_device(dev);
        }
    }

    fn sys_put_device(&self, dev: *mut device) {
        unsafe { kernel::bindings::put_device(dev) }
    }

    fn sys_request_threaded_irq(
        &self,
        irq: c_uint,
        handler: irq_handler_t,
        thread_fn: irq_handler_t,
        flags: c_ulong,
        name: *const c_char,
        dev_id: *mut c_void,
    ) -> c_int {
        unsafe {
            kernel::bindings::request_threaded_irq(irq, handler, thread_fn, flags, name, dev_id)
        }
    }

    fn sys_free_irq(&self, arg1: c_uint, arg2: *mut c_void) -> *const c_void {
        unsafe { kernel::bindings::free_irq(arg1, arg2) }
    }

    fn sys_dma_alloc_attrs(
        &self,
        dev: *mut device,
        size: usize,
        dma_handle: *mut dma_addr_t,
        flag: gfp_t,
        attrs: c_ulong,
    ) -> *mut c_void {
        unsafe { kernel::bindings::dma_alloc_attrs(dev, size, dma_handle, flag, attrs) }
    }

    fn sys_dma_free_attrs(
        &self,
        dev: *mut device,
        size: usize,
        cpu_addr: *mut c_void,
        dma_handle: u64,
        attrs: c_ulong,
    ) {
        unsafe { kernel::bindings::dma_free_attrs(dev, size, cpu_addr, dma_handle, attrs) }
    }

    fn sys_dma_pool_create(
        &self,
        name: *const c_char,
        dev: *mut device,
        size: usize,
        align: usize,
        boundary: usize,
    ) -> *mut dma_pool {
        unsafe { kernel::bindings::dma_pool_create(name, dev, size, align, boundary) }
    }

    fn sys_dma_pool_alloc(
        &self,
        pool: *mut dma_pool,
        flag: gfp_t,
        dma_handle: *mut dma_addr_t,
    ) -> *mut c_void {
        unsafe { kernel::bindings::dma_pool_alloc(pool, flag, dma_handle) }
    }

    fn sys_dma_pool_free(&self, pool: *mut dma_pool, vaddr: *mut c_void, dma_handle: dma_addr_t) {
        unsafe { kernel::bindings::dma_pool_free(pool, vaddr, dma_handle) }
    }

    fn sys_dma_pool_destroy(&self, pool: *mut dma_pool) {
        unsafe { kernel::bindings::dma_pool_destroy(pool) }
    }

    fn sys_ioremap(&self, offset: resource_size_t, size: c_ulong) -> *mut c_void {
        unsafe { kernel::bindings::ioremap(offset, size) }
    }

    fn sys_memcpy_fromio(&self, arg1: *mut c_void, arg2: *const c_void, arg3: usize) {
        unsafe { kernel::bindings::memcpy_fromio(arg1, arg2, arg3) }
    }

    fn sys_iounmap(&self, addr: *mut c_void) {
        unsafe { kernel::bindings::iounmap(addr) }
    }

    fn sys_readb(&self, addr: *const c_void) -> u8_ {
        unsafe { kernel::bindings::readb(addr) }
    }

    fn sys_readw(&self, addr: *const c_void) -> u16_ {
        unsafe { kernel::bindings::readw(addr) }
    }

    fn sys_readl(&self, addr: *const c_void) -> u32_ {
        unsafe { kernel::bindings::readl(addr) }
    }

    fn sys_readq(&self, addr: *const c_void) -> u64_ {
        unsafe { kernel::bindings::readq(addr) }
    }

    fn sys_writeb(&self, value: u8_, addr: *mut c_void) {
        unsafe { kernel::bindings::writeb(value, addr) }
    }

    fn sys_writew(&self, value: u16_, addr: *mut c_void) {
        unsafe { kernel::bindings::writew(value, addr) }
    }

    fn sys_writel(&self, value: u32_, addr: *mut c_void) {
        unsafe { kernel::bindings::writel(value, addr) }
    }

    fn sys_writeq(&self, value: u64_, addr: *mut c_void) {
        unsafe { kernel::bindings::writeq(value, addr) }
    }

    fn sys_readb_relaxed(&self, addr: *const c_void) -> u8_ {
        unsafe { kernel::bindings::readb_relaxed(addr) }
    }

    fn sys_readw_relaxed(&self, addr: *const c_void) -> u16_ {
        unsafe { kernel::bindings::readw_relaxed(addr) }
    }

    fn sys_readl_relaxed(&self, addr: *const c_void) -> u32_ {
        unsafe { kernel::bindings::readl_relaxed(addr) }
    }

    fn sys_readq_relaxed(&self, addr: *const c_void) -> u64_ {
        unsafe { kernel::bindings::readq_relaxed(addr) }
    }

    fn sys_writeb_relaxed(&self, value: u8_, addr: *mut c_void) {
        unsafe { kernel::bindings::writeb_relaxed(value, addr) }
    }

    fn sys_writew_relaxed(&self, value: u16_, addr: *mut c_void) {
        unsafe { kernel::bindings::writew_relaxed(value, addr) }
    }

    fn sys_writel_relaxed(&self, value: u32_, addr: *mut c_void) {
        unsafe { kernel::bindings::writel_relaxed(value, addr) }
    }

    fn sys_writeq_relaxed(&self, value: u64_, addr: *mut c_void) {
        unsafe { kernel::bindings::writeq_relaxed(value, addr) }
    }

    fn sys__pci_register_driver(
        &self,
        arg1: *mut pci_driver,
        arg2: *mut module,
        mod_name: *const c_char,
    ) -> c_int {
        unsafe { kernel::bindings::__pci_register_driver(arg1, arg2, mod_name) }
    }

    fn sys_pci_unregister_driver(&self, arg1: *mut pci_driver) {
        unsafe { kernel::bindings::pci_unregister_driver(arg1) }
    }

    fn sys_pci_set_drvdata(&self, pdev: *mut pci_dev, data: *mut c_void) {
        unsafe { kernel::bindings::pci_set_drvdata(pdev, data) }
    }

    fn sys_pci_get_drvdata(&self, pdev: *mut pci_dev) -> *mut c_void {
        unsafe { kernel::bindings::pci_get_drvdata(pdev) }
    }

    fn sys_pci_enable_device_mem(&self, pdev: *mut pci_dev) -> c_int {
        unsafe { kernel::bindings::pci_enable_device_mem(pdev) }
    }

    fn sys_pci_set_master(&self, pdev: *mut pci_dev) {
        unsafe { kernel::bindings::pci_set_master(pdev) }
    }

    fn sys_pci_select_bars(&self, dev: *mut pci_dev, flags: c_ulong) -> c_int {
        unsafe { kernel::bindings::pci_select_bars(dev, flags) }
    }

    fn sys_pci_request_selected_regions(
        &self,
        arg1: *mut pci_dev,
        arg2: c_int,
        arg3: *const c_char,
    ) -> c_int {
        unsafe { kernel::bindings::pci_request_selected_regions(arg1, arg2, arg3) }
    }

    fn sys_pci_alloc_irq_vectors_affinity(
        &self,
        dev: *mut pci_dev,
        min_vecs: c_uint,
        max_vecs: c_uint,
        flags: c_uint,
        affd: *mut irq_affinity,
    ) -> c_int {
        unsafe {
            kernel::bindings::pci_alloc_irq_vectors_affinity(dev, min_vecs, max_vecs, flags, affd)
        }
    }

    fn sys_pci_free_irq_vectors(&self, pdev: *mut pci_dev) {
        unsafe { kernel::bindings::pci_free_irq_vectors(pdev) }
    }

    fn sys_pci_irq_vector(&self, pdev: *mut pci_dev, nr: c_uint) -> c_int {
        unsafe { kernel::bindings::pci_irq_vector(pdev, nr) }
    }

    fn sys_blk_mq_pci_map_queues(
        &self,
        qmap: *mut blk_mq_queue_map,
        pdev: *mut pci_dev,
        offset: c_int,
    ) {
        unsafe { kernel::bindings::blk_mq_pci_map_queues(qmap, pdev, offset) }
    }

    fn sys_blk_mq_map_queues(&self, qmap: *mut blk_mq_queue_map) {
        unsafe { kernel::bindings::blk_mq_map_queues(qmap) }
    }

    fn sys_dma_map_page_attrs(
        &self,
        dev: *mut device,
        page: *mut page,
        offset: usize,
        size: usize,
        dir: dma_data_direction,
        attrs: c_ulong,
    ) -> dma_addr_t {
        unsafe { kernel::bindings::dma_map_page_attrs(dev, page, offset, size, dir, attrs) }
    }

    fn sys_dma_unmap_page_attrs(
        &self,
        dev: *mut device,
        handle: dma_addr_t,
        size: usize,
        dir: dma_data_direction,
        attrs: c_ulong,
    ) {
        unsafe { kernel::bindings::dma_unmap_page_attrs(dev, handle, size, dir, attrs) }
    }

    fn sys_num_possible_cpus(&self) -> c_uint {
        unsafe { kernel::bindings::num_possible_cpus() }
    }
    fn sys_mdelay(&self, msecs: u64) {
        unsafe { kernel::bindings::mdelay(msecs) }
    }
}

static BLK_CRASH: AtomicBool = AtomicBool::new(true);
fn unwind() {
    BLK_CRASH.store(false, core::sync::atomic::Ordering::Relaxed);
}
