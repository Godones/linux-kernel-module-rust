use core::marker::PhantomData;

use kbind::safe_ptr::SafePtr;
use pinned_init::PinInit;

use crate::{
    bindings,
    kernel::{
        block::mq::{MqOperations, Request, TagSet},
        error::{from_result, KernelResult},
        types::ForeignOwnable,
    },
};

pub struct OperationsConverter<T: MqOperations>(PhantomData<T>);

impl<T: MqOperations> OperationsConverter<T> {
    unsafe fn queue_rq_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        bd: *const bindings::blk_mq_queue_data,
        driver_data: *mut core::ffi::c_void,
    ) -> KernelResult {
        // SAFETY: `bd` is valid as required by the safety requirement for this function.
        let rq = unsafe { Request::from_ptr((*bd).rq) };

        // SAFETY: The safety requirement for this function ensure that
        // `(*hctx).driver_data` was returned by a call to
        // `Self::init_hctx_callback()`. That function uses
        // `PointerWrapper::into_pointer()` to create `driver_data`. Further,
        // the returned value does not outlive this function and
        // `from_foreign()` is not called until `Self::exit_hctx_callback()` is
        // called. By the safety requirement of this function and contract with
        // the `blk-mq` API, `queue_rq_callback()` will not be called after that
        // point.
        let hw_data = unsafe { T::HwData::borrow(driver_data) };

        // SAFETY: `hctx` is valid as required by this function.
        let queue_data = unsafe { (*(*hctx).queue).queuedata };

        // SAFETY: `queue.queuedata` was created by `GenDisk::try_new()` with a
        // call to `ForeignOwnable::into_pointer()` to create `queuedata`.
        // `ForeignOwnable::from_foreign()` is only called when the tagset is
        // dropped, which happens after we are dropped.
        let queue_data = unsafe { T::QueueData::borrow(queue_data) };

        let ret = T::queue_rq(
            hw_data,
            queue_data,
            rq,
            // SAFETY: `bd` is valid as required by the safety requirement for this function.
            unsafe { (*bd).last },
        );
        ret
    }
    unsafe fn init_hctx_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        tagset_data: *mut core::ffi::c_void,
        hctx_idx: core::ffi::c_uint,
    ) -> core::ffi::c_int {
        from_result(|| {
            let tagset_data = unsafe { T::TagSetData::borrow(tagset_data) };
            let data = T::init_hctx(tagset_data, hctx_idx)?;
            unsafe { (*hctx).driver_data = data.into_foreign() as _ };
            Ok(0)
        })
    }

    unsafe fn exit_hctx_callback(hctx: *mut bindings::blk_mq_hw_ctx, _hctx_idx: core::ffi::c_uint) {
        let ptr = unsafe { (*hctx).driver_data };
        unsafe { T::HwData::from_foreign(ptr) };
    }

    unsafe fn init_request_callback(
        _set: *mut bindings::blk_mq_tag_set,
        rq: *mut bindings::request,
        _hctx_idx: core::ffi::c_uint,
        _numa_node: core::ffi::c_uint,
        driver_data: *mut core::ffi::c_void,
    ) -> KernelResult<()> {
        // SAFETY: The tagset invariants guarantee that all requests are allocated with extra memory
        // for the request data.
        let pdu = crate::sys_blk_mq_rq_to_pdu(rq) as *mut T::RequestData;
        let tagset_data = unsafe { T::TagSetData::borrow(driver_data) };

        let initializer = T::new_request_data(tagset_data);
        unsafe { initializer.__pinned_init(pdu)? };
        Ok(())
    }

    unsafe fn exit_request_callback(
        _set: *mut bindings::blk_mq_tag_set,
        rq: *mut bindings::request,
        _hctx_idx: core::ffi::c_uint,
    ) {
        // SAFETY: The tagset invariants guarantee that all requests are allocated with extra memory
        // for the request data.
        let pdu = crate::sys_blk_mq_rq_to_pdu(rq) as *mut T::RequestData;

        // SAFETY: `pdu` is valid for read and write and is properly initialised.
        unsafe { core::ptr::drop_in_place(pdu) };
    }

    unsafe fn commit_rqs_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        driver_data: *mut core::ffi::c_void,
    ) {
        let hw_data = unsafe { T::HwData::borrow(driver_data) };

        // SAFETY: `hctx` is valid as required by this function.
        let queue_data = unsafe { (*(*hctx).queue).queuedata };

        // SAFETY: `queue.queuedata` was created by `GenDisk::try_new()` with a
        // call to `ForeignOwnable::into_pointer()` to create `queuedata`.
        // `ForeignOwnable::from_foreign()` is only called when the tagset is
        // dropped, which happens after we are dropped.
        let queue_data = unsafe { T::QueueData::borrow(queue_data) };
        T::commit_rqs(hw_data, queue_data)
    }

    unsafe fn complete_callback(rq: *mut bindings::request) {
        T::complete(unsafe { &Request::from_ptr(rq) });
    }

    unsafe fn map_queues_callback(
        tag_set: *mut bindings::blk_mq_tag_set,
        driver_data: *mut core::ffi::c_void,
    ) {
        // SAFETY: The safety requirements of this function satiesfies the
        // requirements of `TagSet::from_ptr`.
        let tag_set = unsafe { TagSet::from_ptr(tag_set) };
        let driver_data = unsafe { T::TagSetData::from_foreign(driver_data) };
        T::map_queues(tag_set, driver_data);
    }

    unsafe fn poll_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        _iob: *mut bindings::io_comp_batch,
    ) -> core::ffi::c_int {
        // SAFETY: By function safety requirement, `hctx` was initialized by
        // `init_hctx_callback` and thus `driver_data` came from a call to
        // `into_foreign`.
        let hw_data = unsafe { T::HwData::borrow((*hctx).driver_data) };
        T::poll(hw_data).into()
    }

    pub fn queue_rq(
        hctx_ptr: SafePtr,
        bd_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
    ) -> KernelResult {
        unsafe {
            Self::queue_rq_callback(
                hctx_ptr.raw_ptr() as _,
                bd_ptr.raw_ptr() as _,
                hctx_driver_data_ptr.raw_ptr() as _,
            )
        }
    }

    pub fn init_request(
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        driver_data_ptr: SafePtr,
    ) -> KernelResult {
        unsafe {
            Self::init_request_callback(
                tag_set_ptr.raw_ptr() as _,
                rq_ptr.raw_ptr() as _,
                0,
                0,
                driver_data_ptr.raw_ptr() as _,
            )
        }
    }
    pub fn exit_request(tag_set_ptr: SafePtr, rq_ptr: SafePtr) -> KernelResult {
        unsafe {
            Self::exit_request_callback(tag_set_ptr.raw_ptr() as _, rq_ptr.raw_ptr() as _, 0)
        };
        Ok(())
    }

    pub fn init_hctx(
        hctx_ptr: SafePtr,
        tag_set_data_ptr: SafePtr,
        hctx_idx: usize,
    ) -> KernelResult {
        unsafe {
            Self::init_hctx_callback(
                hctx_ptr.raw_ptr() as _,
                tag_set_data_ptr.raw_ptr() as _,
                hctx_idx as _,
            );
        }
        Ok(())
    }

    pub fn exit_hctx(hctx_ptr: SafePtr, hctx_idx: usize) -> KernelResult {
        unsafe { Self::exit_hctx_callback(hctx_ptr.raw_ptr() as _, hctx_idx as _) };
        Ok(())
    }

    pub fn commit_rqs(hctx_ptr: SafePtr, hctx_driver_data_ptr: SafePtr) -> KernelResult {
        unsafe {
            Self::commit_rqs_callback(hctx_ptr.raw_ptr() as _, hctx_driver_data_ptr.raw_ptr() as _)
        }
        Ok(())
    }

    pub fn complete_request(rq_ptr: SafePtr) -> KernelResult {
        unsafe { Self::complete_callback(rq_ptr.raw_ptr() as _) }
        Ok(())
    }

    pub fn map_queues(tag_set_ptr: SafePtr, driver_data_ptr: SafePtr) -> KernelResult {
        unsafe {
            Self::map_queues_callback(tag_set_ptr.raw_ptr() as _, driver_data_ptr.raw_ptr() as _);
        }
        Ok(())
    }

    pub fn poll_queues(hctx_ptr: SafePtr, iob_ptr: SafePtr) -> KernelResult<i32> {
        let res = unsafe { Self::poll_callback(hctx_ptr.raw_ptr() as _, iob_ptr.raw_ptr() as _) };
        Ok(res)
    }
}
