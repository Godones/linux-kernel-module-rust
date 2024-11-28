// SPDX-License-Identifier: GPL-2.0

//! This module provides an interface for blk-mq drivers to implement.
//!
//! C header: [`include/linux/blk-mq.h`](../../include/linux/blk-mq.h)

use alloc::sync::Arc;
use core::marker::PhantomData;

use interface::nvme::BlkMqOp;
use kmacro::vtable;
use pinned_init::PinInit;

use crate::{
    bindings,
    kernel::{
        block::mq::{Request, TagSet},
        error::{from_result, KernelResult as Result},
        types::ForeignOwnable,
    },
};

/// Implement this trait to interface blk-mq as block devices
#[vtable]
pub trait MqOperations: Sized {
    /// Data associated with a request. This data is located next to the request
    /// structure.
    type RequestData: Sized;

    type RequestDataInit: PinInit<Self::RequestData>;

    /// Data associated with the `struct request_queue` that is allocated for
    /// the `GenDisk` associated with this `Operations` implementation.
    type QueueData: ForeignOwnable;

    /// Data associated with a dispatch queue. This is stored as a pointer in
    /// `struct blk_mq_hw_ctx`.
    type HwData: ForeignOwnable;

    /// Data associated with a tag set. This is stored as a pointer in `struct
    /// blk_mq_tag_set`.
    type TagSetData: ForeignOwnable;
    type DomainType: Clone;
    /// Called by the kernel to get an initializer for a `Pin<&mut RequestData>`.
    fn new_request_data(
        //rq: ARef<Request<Self>>,
        tagset_data: <Self::TagSetData as ForeignOwnable>::Borrowed<'_>,
    ) -> Self::RequestDataInit;

    /// Called by the kernel to queue a request with the driver. If `is_last` is
    /// `false`, the driver is allowed to defer commiting the request.
    fn queue_rq(
        hw_data: <Self::HwData as ForeignOwnable>::Borrowed<'_>,
        queue_data: <Self::QueueData as ForeignOwnable>::Borrowed<'_>,
        rq: Request<Self>,
        is_last: bool,
    ) -> Result;

    /// Called by the kernel to indicate that queued requests should be submitted
    fn commit_rqs(
        hw_data: <Self::HwData as ForeignOwnable>::Borrowed<'_>,
        queue_data: <Self::QueueData as ForeignOwnable>::Borrowed<'_>,
    );

    /// Called by the kernel when the request is completed
    fn complete(_rq: &Request<Self>);

    /// Called by the kernel to allocate and initialize a driver specific hardware context data
    fn init_hctx(
        tagset_data: <Self::TagSetData as ForeignOwnable>::Borrowed<'_>,
        hctx_idx: u32,
    ) -> Result<Self::HwData>;

    /// Called by the kernel to poll the device for completed requests. Only used for poll queues.
    /// Called by the kernel to poll the device for completed requests. Only
    /// used for poll queues.
    fn poll(_hw_data: <Self::HwData as ForeignOwnable>::Borrowed<'_>) -> bool {
        unreachable!("poll: {}", crate::kernel::error::VTABLE_DEFAULT_ERROR)
    }

    /// Called by the kernel to map submission queues to CPU cores.
    fn map_queues(_tag_set: &TagSet<Self>, _tagset_data: Self::TagSetData) {
        unreachable!("map_queues: {}", crate::kernel::error::VTABLE_DEFAULT_ERROR)
    }
    // There is no need for exit_request() because `drop` will be called.
}

pub struct TagSetDataShim {
    pub original_tag_data: *mut core::ffi::c_void,
    pub domain: *mut Arc<dyn BlkMqOp>,
}

impl TagSetDataShim {
    unsafe fn from_raw(original_tag_data: *mut core::ffi::c_void) -> &'static Self {
        unsafe { &*(original_tag_data as *const Self) }
    }
    fn domain(&self) -> &Arc<dyn BlkMqOp> {
        unsafe { &*self.domain }
    }
}

pub struct HctxDataShim {
    original_hcxt_data: *mut core::ffi::c_void,
    domain: *const Arc<dyn BlkMqOp>,
}
impl HctxDataShim {
    unsafe fn from_raw(ptr: *mut core::ffi::c_void) -> &'static Self {
        unsafe { &*(ptr as *const Self) }
    }

    fn new(original_hcxt_data: *mut core::ffi::c_void, domain: *const Arc<dyn BlkMqOp>) -> Self {
        Self {
            original_hcxt_data,
            domain,
        }
    }

    fn domain(&self) -> &Arc<dyn BlkMqOp> {
        unsafe { &*self.domain }
    }
}

pub use shim::OperationsVtableShim;

mod shim {
    use alloc::boxed::Box;
    use core::{marker::PhantomData, sync::atomic::AtomicBool};

    use kbind::safe_ptr::SafePtr;

    use super::{HctxDataShim, TagSetDataShim};
    use crate::{
        bindings,
        kernel::{block::mq::MqOperations, error::Error},
    };

    pub struct OperationsVtableShim<const IO: bool, T: MqOperations>(PhantomData<T>);

    impl<const IO: bool, T: MqOperations> OperationsVtableShim<IO, T> {
        unsafe extern "C" fn queue_rq_callback(
            hctx: *mut bindings::blk_mq_hw_ctx,
            bd: *const bindings::blk_mq_queue_data,
        ) -> bindings::blk_status_t {
            let driver_data = unsafe { HctxDataShim::from_raw((*hctx).driver_data) };
            let domain = driver_data.domain();
            let original_data = driver_data.original_hcxt_data;
            let result = domain.queue_rq(
                SafePtr::new(hctx as _),
                SafePtr::new(bd as *mut bindings::blk_mq_queue_data),
                SafePtr::new(original_data),
                IO,
            );
            match result {
                Ok(()) => bindings::BLK_STS_OK as _,
                Err(e) => Error::from_errno(e as i32).to_blk_status(),
            }
        }
        unsafe extern "C" fn commit_rqs_callback(hctx: *mut bindings::blk_mq_hw_ctx) {
            let driver_data = unsafe { HctxDataShim::from_raw((*hctx).driver_data) };
            let domain = driver_data.domain();
            let original_data = driver_data.original_hcxt_data;
            let _res = domain.commit_rqs(SafePtr::new(hctx as _), SafePtr::new(original_data), IO);
        }
        unsafe extern "C" fn complete_callback(rq: *mut bindings::request) {
            let hctx = (*rq).mq_hctx;
            let driver_data = unsafe { HctxDataShim::from_raw((*hctx).driver_data) };
            let domain = driver_data.domain();
            let _res = domain.complete_request(SafePtr::new(rq as _), IO);
        }

        unsafe extern "C" fn init_hctx_callback(
            hctx: *mut bindings::blk_mq_hw_ctx,
            tagset_data: *mut core::ffi::c_void,
            hctx_idx: core::ffi::c_uint,
        ) -> core::ffi::c_int {
            let driver_data = unsafe { TagSetDataShim::from_raw(tagset_data) };
            let domain = driver_data.domain();
            let original_data = driver_data.original_tag_data;
            let result = domain.init_hctx(
                SafePtr::new(hctx as _),
                SafePtr::new(original_data),
                hctx_idx as usize,
                IO,
            );
            match result {
                Ok(()) => {
                    // update hctx driver data
                    unsafe {
                        let hctx = &mut *hctx;
                        let original_data = hctx.driver_data;
                        let hctx_data = HctxDataShim::new(original_data, driver_data.domain);
                        let hctx_data_ptr = Box::into_raw(Box::new(hctx_data));
                        hctx.driver_data = hctx_data_ptr as _;
                    }
                    0
                }
                Err(e) => e as i32,
            }
        }
        unsafe extern "C" fn exit_hctx_callback(
            hctx: *mut bindings::blk_mq_hw_ctx,
            hctx_idx: core::ffi::c_uint,
        ) {
            let (driver_data, hctx_mut) = unsafe {
                (
                    Box::from_raw((*hctx).driver_data as *mut HctxDataShim),
                    &mut *hctx,
                )
            };
            let original_data = driver_data.original_hcxt_data;
            let domain = driver_data.domain();
            // restore original data
            hctx_mut.driver_data = original_data;
            let _res = domain.exit_hctx(SafePtr::new(hctx as _), hctx_idx as usize, IO);
        }

        unsafe extern "C" fn init_request_callback(
            set: *mut bindings::blk_mq_tag_set,
            rq: *mut bindings::request,
            _hctx_idx: core::ffi::c_uint,
            _numa_node: core::ffi::c_uint,
        ) -> core::ffi::c_int {
            let driver_data = unsafe { TagSetDataShim::from_raw((*set).driver_data) };
            let domain = driver_data.domain();
            let result = domain.init_request(
                SafePtr::new(set as _),
                SafePtr::new(rq as _),
                SafePtr::new(driver_data.original_tag_data),
                IO,
            );
            match result {
                Ok(()) => 0,
                Err(e) => e as i32,
            }
        }
        unsafe extern "C" fn exit_request_callback(
            set: *mut bindings::blk_mq_tag_set,
            rq: *mut bindings::request,
            _hctx_idx: core::ffi::c_uint,
        ) {
            let driver_data = unsafe { TagSetDataShim::from_raw((*set).driver_data) };
            let domain = driver_data.domain();
            let _res = domain.exit_request(SafePtr::new(set as _), SafePtr::new(rq as _), IO);
        }

        unsafe extern "C" fn map_queues_callback(tag_set: *mut bindings::blk_mq_tag_set) {
            let driver_data = unsafe { TagSetDataShim::from_raw((*tag_set).driver_data) };
            let domain = driver_data.domain();
            let _res = domain.map_queues(
                SafePtr::new(tag_set as _),
                SafePtr::new(driver_data.original_tag_data),
                IO,
            );
        }

        unsafe extern "C" fn poll_callback(
            hctx: *mut bindings::blk_mq_hw_ctx,
            iob: *mut bindings::io_comp_batch,
        ) -> core::ffi::c_int {
            let driver_data = unsafe { HctxDataShim::from_raw((*hctx).driver_data) };
            let domain = driver_data.domain();
            let hctx_driver_data_ptr = driver_data.original_hcxt_data;
            let res = domain.poll_queues(
                SafePtr::new(hctx as _),
                SafePtr::new(iob as _),
                SafePtr::new(hctx_driver_data_ptr as _),
                IO,
            );
            res.unwrap_or_else(|e| e as i32)
        }

        const VTABLE: bindings::blk_mq_ops = bindings::blk_mq_ops {
            queue_rq: Some(Self::queue_rq_callback),
            queue_rqs: None,
            commit_rqs: Some(Self::commit_rqs_callback),
            get_budget: None,
            put_budget: None,
            set_rq_budget_token: None,
            get_rq_budget_token: None,
            timeout: None,
            poll: if T::HAS_POLL {
                Some(Self::poll_callback)
            } else {
                None
            },
            complete: Some(Self::complete_callback),
            init_hctx: Some(Self::init_hctx_callback),
            exit_hctx: Some(Self::exit_hctx_callback),
            init_request: Some(Self::init_request_callback),
            exit_request: Some(Self::exit_request_callback),
            cleanup_rq: None,
            busy: None,
            map_queues: if T::HAS_MAP_QUEUES {
                Some(Self::map_queues_callback)
            } else {
                None
            },
            show_rq: None,
        };

        pub(crate) const unsafe fn build() -> &'static bindings::blk_mq_ops {
            &Self::VTABLE
        }
    }
}
