use alloc::{boxed::Box, sync::Arc};
use core::{any::Any, sync::atomic::AtomicUsize};

use corelib::SafePtr;
use interface::{null_block::BlockDeviceDomain, DomainTypeRaw};
use kernel::{
    bindings,
    error::{from_err_ptr, Error, KernelResult},
};

use crate::kshim::KernelShim;

pub struct BlockDeviceShim {
    domain_ptr: *const Arc<dyn BlockDeviceDomain>,
    gendisk: *mut bindings::gendisk,
    tagset: *mut bindings::blk_mq_tag_set,
    domain_type: DomainTypeRaw,
}

impl KernelShim for BlockDeviceShim {
    fn any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn domain_type(&self) -> DomainTypeRaw {
        self.domain_type
    }
}

unsafe impl Send for BlockDeviceShim {}
unsafe impl Sync for BlockDeviceShim {}

struct TagSetData {
    original_data: *mut core::ffi::c_void,
    domain: *const Arc<dyn BlockDeviceDomain>,
}

struct HctxData {
    original_data: *mut core::ffi::c_void,
    domain: *const Arc<dyn BlockDeviceDomain>,
}

impl HctxData {
    unsafe fn from_raw(ptr: *mut core::ffi::c_void) -> &'static Self {
        unsafe { &*(ptr as *const Self) }
    }

    fn new(
        original_data: *mut core::ffi::c_void,
        domain: *const Arc<dyn BlockDeviceDomain>,
    ) -> Self {
        Self {
            original_data,
            domain,
        }
    }

    fn domain(&self) -> &Arc<dyn BlockDeviceDomain> {
        unsafe { &*self.domain }
    }
}

impl TagSetData {
    unsafe fn from_raw(ptr: *mut core::ffi::c_void) -> &'static Self {
        unsafe { &*(ptr as *const Self) }
    }
    fn domain(&self) -> &Arc<dyn BlockDeviceDomain> {
        unsafe { &*self.domain }
    }
}

impl BlockDeviceShim {
    pub fn load(domain: Arc<dyn BlockDeviceDomain>) -> KernelResult<Self> {
        let (tag_set_ptr, queue_data_ptr) = domain
            .tag_set_with_queue_data()
            .map_err(|e| Error::from_errno(e as i32))?;
        let tagset_ptr = unsafe { tag_set_ptr.raw_ptr() as *mut bindings::blk_mq_tag_set };
        let tagset = unsafe { &mut *tagset_ptr };

        let domain_ptr = Box::into_raw(Box::new(domain.clone()));

        let tagset_data = TagSetData {
            original_data: tagset.driver_data,
            domain: domain_ptr,
        };
        tagset.driver_data = Box::into_raw(Box::new(tagset_data)) as _;
        tagset.ops = &TAGSET_OPS_TABLE;

        let ret = unsafe { bindings::blk_mq_alloc_tag_set(tagset_ptr) };

        if ret < 0 {
            return Err(Error::from_errno(ret));
        }

        let queue_data_ptr = unsafe { queue_data_ptr.raw_ptr() };

        let gendisk = Self::alloc_gen_disk(tagset_ptr, queue_data_ptr)?;

        let (gen_disk, gen_disk_sptr) = unsafe { (&mut *gendisk, SafePtr::new(gendisk as _)) };

        gen_disk.private_data = domain_ptr as _;
        gen_disk.fops = &DISK_OPS_TABLE;

        domain
            .set_gen_disk(gen_disk_sptr)
            .map_err(|e| Error::from_errno(e as i32))?;

        let block_device_shim = Self {
            domain_ptr,
            gendisk,
            domain_type: DomainTypeRaw::BlockDeviceDomain,
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
        // SAFETY: `tagset.raw_tag_set()` points to a valid and initialized tag set
        let gendisk = from_err_ptr(unsafe {
            bindings::__blk_mq_alloc_disk(tagset, queue_data as _, lock_class_key.as_ptr())
        })?;
        Ok(gendisk)
    }

    /// Register the block device with the kernel
    fn add_disk(&self) -> KernelResult<()> {
        kernel::error::to_result(unsafe {
            bindings::device_add_disk(core::ptr::null_mut(), self.gendisk, core::ptr::null_mut())
        })?;

        Ok(())
    }
}

impl Drop for BlockDeviceShim {
    fn drop(&mut self) {
        unsafe {
            // release the domain
            bindings::del_gendisk(self.gendisk);
            // SAFETY: `inner` is valid and has been properly initialised during construction.
            bindings::blk_mq_free_tag_set(self.tagset);
            let tagset = &mut *self.tagset;
            let tagset_data = Box::from_raw(tagset.driver_data as *mut TagSetData);
            let original_data = tagset_data.original_data;
            drop(tagset_data);
            // restore original data, domain will drop it
            tagset.driver_data = original_data;
        }
        let domain = unsafe { Box::from_raw(self.domain_ptr as *mut Arc<dyn BlockDeviceDomain>) };

        domain
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

    use corelib::SafePtr;
    use kernel::{bindings, error::Error};

    use crate::kshim::block_device::{HctxData, TagSetData};

    pub unsafe extern "C" fn queue_rq_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        bd: *const bindings::blk_mq_queue_data,
    ) -> bindings::blk_status_t {
        let driver_data = unsafe { HctxData::from_raw((*hctx).driver_data) };
        let domain = driver_data.domain();
        let original_data = driver_data.original_data;
        let result = domain.queue_rq(
            SafePtr::new(hctx as _),
            SafePtr::new(bd as _),
            SafePtr::new(original_data),
        );
        match result {
            Ok(()) => bindings::BLK_STS_OK as _,
            Err(e) => Error::from_errno(e as i32).to_blk_status(),
        }
    }

    pub unsafe extern "C" fn commit_rqs_callback(hctx: *mut bindings::blk_mq_hw_ctx) {
        let driver_data = unsafe { HctxData::from_raw((*hctx).driver_data) };
        let domain = driver_data.domain();
        let original_data = driver_data.original_data;
        let _res = domain.commit_rqs(SafePtr::new(hctx as _), SafePtr::new(original_data));
    }
    pub unsafe extern "C" fn complete_callback(rq: *mut bindings::request) {
        let hctx = (*rq).mq_hctx;
        let driver_data = unsafe { HctxData::from_raw((*hctx).driver_data) };
        let domain = driver_data.domain();
        let _res = domain.complete_request(SafePtr::new(rq as _));
    }

    pub unsafe extern "C" fn init_hctx_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        tagset_data: *mut core::ffi::c_void,
        hctx_idx: core::ffi::c_uint,
    ) -> core::ffi::c_int {
        let driver_data = unsafe { TagSetData::from_raw(tagset_data) };
        let domain = driver_data.domain();
        let original_data = driver_data.original_data;
        let result = domain.init_hctx(
            SafePtr::new(hctx as _),
            SafePtr::new(original_data),
            hctx_idx as usize,
        );
        match result {
            Ok(()) => {
                // update hctx driver data
                unsafe {
                    let hctx = &mut *hctx;
                    let original_data = hctx.driver_data;
                    let hctx_data = HctxData::new(original_data, driver_data.domain);
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
        let (driver_data, hctx_mut) = unsafe {
            (
                Box::from_raw((*hctx).driver_data as *mut HctxData),
                &mut *hctx,
            )
        };
        let original_data = driver_data.original_data;
        let domain = driver_data.domain();
        // restore original data
        hctx_mut.driver_data = original_data;
        let _res = domain.exit_hctx(SafePtr::new(hctx as _), hctx_idx as usize);
    }

    pub unsafe extern "C" fn init_request_callback(
        set: *mut bindings::blk_mq_tag_set,
        rq: *mut bindings::request,
        _hctx_idx: core::ffi::c_uint,
        _numa_node: core::ffi::c_uint,
    ) -> core::ffi::c_int {
        let driver_data = unsafe { TagSetData::from_raw((*set).driver_data) };
        let domain = driver_data.domain();
        let result = domain.init_request(
            SafePtr::new(set as _),
            SafePtr::new(rq as _),
            SafePtr::new(driver_data.original_data),
        );
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
        let driver_data = unsafe { TagSetData::from_raw((*set).driver_data) };
        let domain = driver_data.domain();
        let _res = domain.exit_request(SafePtr::new(set as _), SafePtr::new(rq as _));
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
