use alloc::{string::ToString, sync::Arc};
use core::{
    any::Any,
    ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void},
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
}

static BLK_CRASH: AtomicBool = AtomicBool::new(true);
fn unwind() {
    BLK_CRASH.store(false, core::sync::atomic::Ordering::Relaxed);
}
