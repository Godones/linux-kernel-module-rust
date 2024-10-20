use alloc::{boxed::Box, sync::Arc};

use interface::null_block::BlockDeviceDomain;
use kernel::{
    bindings,
    error::{from_err_ptr, Error, KernelResult},
};

pub struct BlockDeviceShim {
    domain: Arc<dyn BlockDeviceDomain>,
    gendisk: *mut bindings::gendisk,
    tagset: *mut bindings::blk_mq_tag_set,
}

struct TagSetData {
    original_data: *mut core::ffi::c_void,
    domain: Arc<dyn BlockDeviceDomain>,
}

struct HctxData {
    original_data: *mut core::ffi::c_void,
    domain: Arc<dyn BlockDeviceDomain>,
}

impl HctxData {
    pub unsafe fn from_raw(ptr: *mut core::ffi::c_void) -> &'static Self {
        unsafe { &*(ptr as *const Self) }
    }
    pub fn new(original_data: *mut core::ffi::c_void, domain: Arc<dyn BlockDeviceDomain>) -> Self {
        Self {
            original_data,
            domain,
        }
    }
}

impl TagSetData {
    pub unsafe fn from_raw(ptr: *mut core::ffi::c_void) -> &'static Self {
        unsafe { &*(ptr as *const Self) }
    }
}

impl BlockDeviceShim {
    pub fn load(domain: Arc<dyn BlockDeviceDomain>) -> KernelResult<Self> {
        let (tag_set_ptr, queue_data_ptr) = domain
            .tag_set_with_queue_data()
            .map_err(|e| Error::from_errno(e as i32))?;
        let tagset_ptr = tag_set_ptr as *mut bindings::blk_mq_tag_set;
        let tagset = unsafe { &mut *tagset_ptr };

        let tagset_data = TagSetData {
            original_data: tagset.driver_data,
            domain: domain.clone(),
        };
        tagset.driver_data = Box::into_raw(Box::new(tagset_data)) as _;
        tagset.ops = &TAGSET_OPS_TABLE;

        pr_info!("BlockDeviceShim: before blk_mq_alloc_tag_set");
        let ret = unsafe { bindings::blk_mq_alloc_tag_set(tagset_ptr) };
        pr_info!("BlockDeviceShim: after blk_mq_alloc_tag_set");
        if ret < 0 {
            return Err(Error::from_errno(ret));
        }

        let queue_data_ptr = queue_data_ptr as *mut core::ffi::c_void;

        let gendisk = Self::alloc_gen_disk(tagset_ptr, queue_data_ptr)?;

        let gen_disk = unsafe { &mut *gendisk };

        gen_disk.private_data = Box::into_raw(Box::new(domain.clone())) as _;
        gen_disk.fops = &DISK_OPS_TABLE;

        domain
            .set_gen_disk(gendisk as usize)
            .map_err(|e| Error::from_errno(e as i32))?;

        let block_device_shim = Self {
            domain,
            gendisk,
            tagset: tagset_ptr,
        };

        block_device_shim.add_disk()?;
        Ok(block_device_shim)
    }

    /// Allocate a generic disk
    fn alloc_gen_disk(
        tagset: *mut bindings::blk_mq_tag_set,
        queue_data: *mut core::ffi::c_void,
    ) -> KernelResult<*mut bindings::gendisk> {
        let lock_class_key = kernel::sync::LockClassKey::new();
        pr_info!("BlockDeviceShim: before __blk_mq_alloc_disk");
        // SAFETY: `tagset.raw_tag_set()` points to a valid and initialized tag set
        let gendisk = from_err_ptr(unsafe {
            bindings::__blk_mq_alloc_disk(tagset, queue_data as _, lock_class_key.as_ptr())
        })?;
        pr_info!("BlockDeviceShim: after __blk_mq_alloc_disk");
        Ok(gendisk)
    }

    /// Register the block device with the kernel
    fn add_disk(&self) -> KernelResult<()> {
        pr_info!("BlockDeviceShim: before device_add_disk");
        kernel::error::to_result(unsafe {
            bindings::device_add_disk(core::ptr::null_mut(), self.gendisk, core::ptr::null_mut())
        })?;
        pr_info!("BlockDeviceShim: after device_add_disk");
        Ok(())
    }
}

impl Drop for BlockDeviceShim {
    fn drop(&mut self) {
        unsafe {
            // release the domain
            let gen_disk = &mut *self.gendisk;
            let domain = Box::from_raw(gen_disk.private_data as *mut Arc<dyn BlockDeviceDomain>);
            drop(domain);

            pr_info!("BlockDeviceShim: before del_gendisk");
            bindings::del_gendisk(self.gendisk);
            pr_info!("BlockDeviceShim: after del_gendisk");

            pr_info!("BlockDeviceShim: before blk_mq_free_tag_set");
            // SAFETY: `inner` is valid and has been properly initialised during construction.
            bindings::blk_mq_free_tag_set(self.tagset);
            pr_info!("BlockDeviceShim: after blk_mq_free_tag_set");

            let tagset = &mut *self.tagset;
            let tagset_data = Box::from_raw(tagset.driver_data as *mut TagSetData);
            let original_data = tagset_data.original_data;
            drop(tagset_data);
            // restore original data, domain will drop it
            tagset.driver_data = original_data;
        }

        self.domain
            .exit()
            .map_err(|e| pr_err!("BlockDeviceShim: domain exit error: {}", e))
            .ok();
    }
}

mod block_ops {
    use alloc::sync::Arc;

    use interface::null_block::BlockDeviceDomain;
    use kernel::bindings;
    pub unsafe extern "C" fn open(
        disk: *mut bindings::gendisk,
        mode: bindings::blk_mode_t,
    ) -> core::ffi::c_int {
        let private_data = (*disk).private_data;
        let block_device_domain = &*(private_data as *const Arc<dyn BlockDeviceDomain>);
        let result = block_device_domain.open(mode);
        match result {
            Ok(()) => 0,
            Err(e) => e as i32,
        }
    }
    pub unsafe extern "C" fn release(disk: *mut bindings::gendisk) {
        let private_data = (*disk).private_data;
        let block_device_domain = &*(private_data as *const Arc<dyn BlockDeviceDomain>);
        let _result = block_device_domain.release();
    }
}

mod block_mq_ops {
    use alloc::boxed::Box;
    use core::sync::atomic::AtomicUsize;

    use kernel::{bindings, error::Error};

    use crate::kshim::block_device::{HctxData, TagSetData};

    pub unsafe extern "C" fn queue_rq_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        bd: *const bindings::blk_mq_queue_data,
    ) -> bindings::blk_status_t {
        pr_info!("BlockDeviceShim: queue_rq_callback began");
        let driver_data = unsafe { HctxData::from_raw((*hctx).driver_data) };
        let domain = &driver_data.domain;
        let original_data = driver_data.original_data;
        let result = domain.queue_rq(hctx as usize, bd as usize, original_data as usize);
        pr_info!("BlockDeviceShim: queue_rq_callback ended");
        match result {
            Ok(()) => bindings::BLK_STS_OK as _,
            Err(e) => Error::from_errno(e as i32).to_blk_status(),
        }
    }

    pub unsafe extern "C" fn commit_rqs_callback(hctx: *mut bindings::blk_mq_hw_ctx) {}
    pub unsafe extern "C" fn complete_callback(rq: *mut bindings::request) {}

    pub unsafe extern "C" fn init_hctx_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        tagset_data: *mut core::ffi::c_void,
        hctx_idx: core::ffi::c_uint,
    ) -> core::ffi::c_int {
        pr_info!(
            "BlockDeviceShim: init_hctx_callback began, hctx: {:?}",
            hctx_idx
        );
        let driver_data = unsafe { TagSetData::from_raw(tagset_data) };
        let domain = &driver_data.domain;
        let original_data = driver_data.original_data;
        let result = domain.init_hctx(hctx as usize, original_data as usize, hctx_idx as usize);
        pr_info!("BlockDeviceShim: init_hctx_callback ended");
        match result {
            Ok(()) => {
                // update hctx driver data
                unsafe {
                    let hctx = &mut *hctx;
                    let original_data = hctx.driver_data;
                    let hctx_data = HctxData::new(original_data, domain.clone());
                    let hctx_data_ptr = Box::into_raw(Box::new(hctx_data));
                    hctx.driver_data = hctx_data_ptr as _;
                }
                0
            }
            Err(e) => e as i32,
        }
    }

    pub unsafe extern "C" fn exit_hctx_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        hctx_idx: core::ffi::c_uint,
    ) {
        pr_info!(
            "BlockDeviceShim: exit_hctx_callback began, hctx: {:?}",
            hctx_idx
        );
        let (driver_data, hctx_mut) = unsafe {
            (
                Box::from_raw((*hctx).driver_data as *mut HctxData),
                &mut *hctx,
            )
        };
        let original_data = driver_data.original_data;
        let domain = &driver_data.domain;
        // restore original data
        hctx_mut.driver_data = original_data;
        let _res = domain.exit_hctx(hctx as usize, hctx_idx as usize);
        pr_info!("BlockDeviceShim: exit_hctx_callback ended");
    }

    pub unsafe extern "C" fn init_request_callback(
        set: *mut bindings::blk_mq_tag_set,
        rq: *mut bindings::request,
        _hctx_idx: core::ffi::c_uint,
        _numa_node: core::ffi::c_uint,
    ) -> core::ffi::c_int {
        static CALL: AtomicUsize = AtomicUsize::new(0);
        pr_info!(
            "BlockDeviceShim: init_request_callback began, call count: {}, {:p}",
            CALL.fetch_add(1, core::sync::atomic::Ordering::Relaxed),
            rq
        );
        let driver_data = unsafe { TagSetData::from_raw((*set).driver_data) };
        let domain = &driver_data.domain;
        let result = domain.init_request(
            set as usize,
            rq as usize,
            driver_data.original_data as usize,
        );
        pr_info!("BlockDeviceShim: init_request_callback ended");
        match result {
            Ok(()) => 0,
            Err(e) => e as i32,
        }
    }
    pub unsafe extern "C" fn exit_request_callback(
        set: *mut bindings::blk_mq_tag_set,
        rq: *mut bindings::request,
        _hctx_idx: core::ffi::c_uint,
    ) {
        pr_info!("BlockDeviceShim: exit_request_callback began, rq: {:p}", rq);
        let driver_data = unsafe { TagSetData::from_raw((*set).driver_data) };
        let domain = &driver_data.domain;
        let _res = domain.exit_request(set as usize, rq as usize);
        pr_info!("BlockDeviceShim: exit_request_callback ended");
    }

    pub unsafe extern "C" fn map_queues_callback(_tag_set_ptr: *mut bindings::blk_mq_tag_set) {}
    pub unsafe extern "C" fn poll_callback(
        _hctx: *mut bindings::blk_mq_hw_ctx,
        _iob: *mut bindings::io_comp_batch,
    ) -> core::ffi::c_int {
        0
    }
}

const DISK_OPS_TABLE: bindings::block_device_operations = bindings::block_device_operations {
    submit_bio: None,
    open: Some(block_ops::open),
    release: Some(block_ops::release),
    ioctl: None,
    compat_ioctl: None,
    check_events: None,
    unlock_native_capacity: None,
    getgeo: None,
    set_read_only: None,
    swap_slot_free_notify: None,
    report_zones: None,
    devnode: None,
    alternative_gpt_sector: None,
    get_unique_id: None,
    owner: core::ptr::null_mut(),
    pr_ops: core::ptr::null_mut(),
    free_disk: None,
    poll_bio: None,
};

const TAGSET_OPS_TABLE: bindings::blk_mq_ops = bindings::blk_mq_ops {
    queue_rq: Some(block_mq_ops::queue_rq_callback),
    queue_rqs: None,
    commit_rqs: Some(block_mq_ops::commit_rqs_callback),
    get_budget: None,
    put_budget: None,
    set_rq_budget_token: None,
    get_rq_budget_token: None,
    timeout: None,
    poll: None,
    complete: Some(block_mq_ops::complete_callback),
    init_hctx: Some(block_mq_ops::init_hctx_callback),
    exit_hctx: Some(block_mq_ops::exit_hctx_callback),
    init_request: Some(block_mq_ops::init_request_callback),
    exit_request: Some(block_mq_ops::exit_request_callback),
    cleanup_rq: None,
    busy: None,
    map_queues: None,
    show_rq: None,
};
