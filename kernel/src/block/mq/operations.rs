// SPDX-License-Identifier: GPL-2.0

//! This module provides an interface for blk-mq drivers to implement.
//!
//! C header: [`include/linux/blk-mq.h`](../../include/linux/blk-mq.h)

use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use kmacro::vtable;

use crate::{
    bindings,
    block::mq::{Request, TagSet},
    error::{from_result, KernelResult as Result},
    init::PinInit,
    pr_info,
    types::{ARef, ForeignOwnable},
};
use crate::block::mq::request::RequestDataWrapper;

/// Implement this trait to interface blk-mq as block devices
#[vtable]
pub trait Operations: Sized {
    /// Data associated with a request. This data is located next to the request
    /// structure.
    type RequestData: Sized + Sync;

    /// Data associated with the `struct request_queue` that is allocated for
    /// the `GenDisk` associated with this `Operations` implementation.
    type QueueData: ForeignOwnable;

    /// Data associated with a dispatch queue. This is stored as a pointer in
    /// `struct blk_mq_hw_ctx`.
    type HwData: ForeignOwnable;

    /// Data associated with a tag set. This is stored as a pointer in `struct
    /// blk_mq_tag_set`.
    type TagSetData: ForeignOwnable;

    /// Called by the kernel to get an initializer for a `Pin<&mut RequestData>`.
    fn new_request_data(
        //rq: ARef<Request<Self>>,
        tagset_data: <Self::TagSetData as ForeignOwnable>::Borrowed<'_>,
    ) -> impl PinInit<Self::RequestData>;

    /// Called by the kernel to queue a request with the driver. If `is_last` is
    /// `false`, the driver is allowed to defer committing the request.
    fn queue_rq(
        hw_data: <Self::HwData as ForeignOwnable>::Borrowed<'_>,
        queue_data: <Self::QueueData as ForeignOwnable>::Borrowed<'_>,
        rq: ARef<Request<Self>>,
        is_last: bool,
    ) -> Result;

    /// Called by the kernel to indicate that queued requests should be submitted
    fn commit_rqs(
        hw_data: <Self::HwData as ForeignOwnable>::Borrowed<'_>,
        queue_data: <Self::QueueData as ForeignOwnable>::Borrowed<'_>,
    );

    /// Called by the kernel when the request is completed.
    fn complete(rq: ARef<Request<Self>>);

    /// Called by the kernel to allocate and initialize a driver specific hardware context data
    fn init_hctx(
        tagset_data: <Self::TagSetData as ForeignOwnable>::Borrowed<'_>,
        hctx_idx: u32,
    ) -> Result<Self::HwData>;

    /// Called by the kernel to poll the device for completed requests. Only used for poll queues.
    /// Called by the kernel to poll the device for completed requests. Only
    /// used for poll queues.
    fn poll(_hw_data: <Self::HwData as ForeignOwnable>::Borrowed<'_>) -> bool {
        unreachable!("poll: {}", crate::error::VTABLE_DEFAULT_ERROR)
    }

    /// Called by the kernel to map submission queues to CPU cores.
    fn map_queues(_tag_set: &TagSet<Self>) {
        unreachable!("map_queues: {}", crate::error::VTABLE_DEFAULT_ERROR)
    }

    // There is no need for exit_request() because `drop` will be called.
}

pub(crate) struct OperationsVtable<T: Operations>(PhantomData<T>);

impl<T: Operations> OperationsVtable<T> {
    // # Safety
    //
    // - The caller of this function must ensure that `hctx` and `bd` are valid
    //   and initialized. The pointees must outlive this function.
    // - `hctx->driver_data` must be a pointer created by a call to
    //   `Self::init_hctx_callback()` and the pointee must outlive this
    //   function.
    // - This function must not be called with a `hctx` for which
    //   `Self::exit_hctx_callback()` has been called.
    // - (*bd).rq must point to a valid `bindings:request` with a positive refcount in the `ref` field.
    unsafe extern "C" fn queue_rq_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        bd: *const bindings::blk_mq_queue_data,
    ) -> bindings::blk_status_t {
        // SAFETY: `bd.rq` is valid as required by the safety requirement for
        // this function.
        let request = unsafe { &*(*bd).rq.cast::<Request<T>>() };

        // One refcount for the ARef, one for being in flight
        request.wrapper_ref().refcount().store(2, Ordering::Relaxed);

        // The request is not completed yet.
        request.wrapper_ref().completed().store(false, Ordering::Relaxed);

        // SAFETY:
        //  - We own a refcount that we took above. We pass that to `ARef`.
        //  - By the safety requirements of this function, `request` is a valid
        //    `struct request` and the private data is properly initialized.
        //  - `rq` will be alive until `blk_mq_end_request` is called and is
        //    reference counted by `ARef` until then.
        let rq = unsafe { Request::aref_from_raw((*bd).rq) };

        // SAFETY: The safety requirement for this function ensure that `hctx`
        // is valid and that `driver_data` was produced by a call to
        // `into_foreign` in `Self::init_hctx_callback`.
        let hw_data = unsafe { T::HwData::borrow((*hctx).driver_data) };

        // SAFETY: `hctx` is valid as required by this function.
        let queue_data = unsafe { (*(*hctx).queue).queuedata };

        // SAFETY: `queue.queuedata` was created by `GenDisk::try_new()` with a
        // call to `ForeignOwnable::into_pointer()` to create `queuedata`.
        // `ForeignOwnable::from_foreign()` is only called when the tagset is
        // dropped, which happens after we are dropped.
        let queue_data = unsafe { T::QueueData::borrow(queue_data) };

        // SAFETY: We have exclusive access and we just set the refcount above.
        unsafe { Request::start_unchecked(&rq) };

        let ret = T::queue_rq(
            hw_data,
            queue_data,
            rq,
            // SAFETY: `bd` is valid as required by the safety requirement for
            // this function.
            unsafe { (*bd).last },
        );
        // pr_info!("queue_rq_callback ended");
        if let Err(e) = ret {
            e.to_blk_status()
        } else {
            bindings::BLK_STS_OK as _
        }
    }

    unsafe extern "C" fn commit_rqs_callback(hctx: *mut bindings::blk_mq_hw_ctx) {
        // pr_info!("commit_rqs_callback began");
        // SAFETY: `driver_data` was installed by us in `init_hctx_callback` as
        // the result of a call to `into_foreign`.
        let hw_data = unsafe { T::HwData::borrow((*hctx).driver_data) };

        // SAFETY: `hctx` is valid as required by this function.
        let queue_data = unsafe { (*(*hctx).queue).queuedata };

        // SAFETY: `queue.queuedata` was created by `GenDisk::try_new()` with a
        // call to `ForeignOwnable::into_pointer()` to create `queuedata`.
        // `ForeignOwnable::from_foreign()` is only called when the tagset is
        // dropped, which happens after we are dropped.
        let queue_data = unsafe { T::QueueData::borrow(queue_data) };
        T::commit_rqs(hw_data, queue_data);
        // pr_info!("commit_rqs_callback ended");
    }

    unsafe extern "C" fn complete_callback(rq: *mut bindings::request) {
        // pr_info!("complete_callback began");
        let aref = unsafe { Request::aref_from_raw(rq) };
        T::complete(aref);
        // pr_info!("complete_callback ended");
    }

    /// # Safety
    ///
    /// This function may only be called by blk-mq C infrastructure. `hctx` must
    /// be a pointer to a valid and aligned `struct blk_mq_hw_ctx` that was
    /// previously initialized by a call to `init_hctx_callback`.
    unsafe extern "C" fn poll_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        _iob: *mut bindings::io_comp_batch,
    ) -> core::ffi::c_int {
        // SAFETY: By function safety requirement, `hctx` was initialized by
        // `init_hctx_callback` and thus `driver_data` came from a call to
        // `into_foreign`.
        let hw_data = unsafe { T::HwData::borrow((*hctx).driver_data) };
        T::poll(hw_data).into()
    }

    unsafe extern "C" fn init_hctx_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        tagset_data: *mut core::ffi::c_void,
        hctx_idx: core::ffi::c_uint,
    ) -> core::ffi::c_int {
        // pr_info!("init_hctx_callback began, hctx: {:?}", hctx_idx);
        let res = from_result(|| {
            let tagset_data = unsafe { T::TagSetData::borrow(tagset_data) };
            let data = T::init_hctx(tagset_data, hctx_idx)?;
            unsafe { (*hctx).driver_data = data.into_foreign() as _ };
            Ok(0)
        });
        // pr_info!("init_hctx_callback ended");
        res
    }

    unsafe extern "C" fn exit_hctx_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        _hctx_idx: core::ffi::c_uint,
    ) {
        pr_info!("exit_hctx_callback began, hctx: {:?}", _hctx_idx);
        let ptr = unsafe { (*hctx).driver_data };
        unsafe { T::HwData::from_foreign(ptr) };
        pr_info!("exit_hctx_callback ended");
    }

    unsafe extern "C" fn init_request_callback(
        set: *mut bindings::blk_mq_tag_set,
        rq: *mut bindings::request,
        _hctx_idx: core::ffi::c_uint,
        _numa_node: core::ffi::c_uint,
    ) -> core::ffi::c_int {
        let res = from_result(|| {
            // SAFETY: By the safety requirements of this function, `rq` points
            // to a valid allocation.
            let pdu = unsafe { Request::wrapper_ptr(rq.cast::<Request<T>>()) };

            // TODO: Perhaps just pin_init `RequestDataWrapper`?

            // SAFETY: The refcount field is allocated but not initialized, so
            // it is valid for writes.
            unsafe { RequestDataWrapper::refcount_ptr(pdu.as_ptr()).write(AtomicU64::new(0)) };

            // SAFETY: The `completed` field is allocated but not initialized,
            // so it is valid for writes.
            unsafe { RequestDataWrapper::completed_ptr(pdu.as_ptr()).write(AtomicBool::new(false)) };

            // SAFETY: Because `set` is a `TagSet<T>`, `driver_data` comes from
            // a call to `into_foregn` by the initializer returned by
            // `TagSet::try_new`.
            let tagset_data = unsafe { T::TagSetData::borrow((*set).driver_data) };

            let initializer = T::new_request_data(tagset_data);

            // SAFETY: `pdu` is a valid pointer as established above. We do not
            // touch `pdu` if `__pinned_init` returns an error. We promise ot to
            // move the pointee of `pdu`.
            unsafe { initializer.__pinned_init(RequestDataWrapper::data_ptr(pdu.as_ptr()))? };

            Ok(0)
        });
        // pr_info!("init_request_callback ended");
        res
    }

    unsafe extern "C" fn exit_request_callback(
        _set: *mut bindings::blk_mq_tag_set,
        rq: *mut bindings::request,
        _hctx_idx: core::ffi::c_uint,
    ) {
        // pr_info!("exit_request_callback began, rq: {:p}", rq);
        // SAFETY: The tagset invariants guarantee that all requests are allocated with extra memory
        // for the request data.
        let pdu = unsafe { bindings::blk_mq_rq_to_pdu(rq) }.cast::<RequestDataWrapper<T>>();

        // SAFETY: `pdu` is valid for read and write and is properly initialised.
        unsafe { core::ptr::drop_in_place(pdu) };

        // pr_info!("exit_request_callback ended");
    }

    /// # Safety
    ///
    /// This function may only be called by blk-mq C infrastructure. `tag_set`
    /// must be a pointer to a valid and initialized `TagSet<T>`. The pointee
    /// must be valid for use as a reference at least the duration of this call.
    unsafe extern "C" fn map_queues_callback(tag_set: *mut bindings::blk_mq_tag_set) {
        // SAFETY: The safety requirements of this function satiesfies the
        // requirements of `TagSet::from_ptr`.
        let tag_set = unsafe { TagSet::from_ptr(tag_set) };
        T::map_queues(tag_set);
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

    pub(crate) const fn build() -> &'static bindings::blk_mq_ops {
        &Self::VTABLE
    }
}
