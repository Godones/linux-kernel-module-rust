// SPDX-License-Identifier: GPL-2.0

//! This module provides the `TagSet` struct to wrap the C `struct blk_mq_tag_set`.
//!
//! C header: [`include/linux/blk-mq.h`](../../include/linux/blk-mq.h)

use alloc::boxed::Box;
use core::{convert::TryInto, ffi::c_void, marker::PhantomData, pin::Pin};

use pinned_init::{pin_data, pinned_drop, try_pin_init, PinInit, PinnedDrop};

use crate::{
    bindings,
    kernel::{
        block::mq::{
            operations::{OperationsVtableShim, TagSetDataShim},
            request::RequestRef,
            MqOperations,
        },
        error::{Error, KernelResult as Result},
        types::{ForeignOwnable, Opaque},
    },
};

/// A wrapper for the C `struct blk_mq_tag_set`.
///
/// `struct blk_mq_tag_set` contains a `struct list_head` and so must be pinned.
#[pin_data(PinnedDrop)]
#[repr(C)]
pub struct TagSet<T: MqOperations> {
    #[pin]
    inner: Opaque<bindings::blk_mq_tag_set>,
    overwrite: bool,
    _p: PhantomData<T>,
}

impl<T: MqOperations> TagSet<T> {
    /// Try to create a new tag set
    pub fn try_new_no_alloc(
        nr_hw_queues: u32,
        tagset_data: T::TagSetData,
        num_tags: u32,
        num_maps: u32,
    ) -> impl PinInit<Self, Error> {
        let res = try_pin_init!( TagSet {
            inner <- Opaque::try_ffi_init(move |place: *mut bindings::blk_mq_tag_set| -> Result<()> {

                // SAFETY: try_ffi_init promises that `place` is writable, and
                // zeroes is a valid bit pattern for this structure.
                unsafe { core::ptr::write_bytes(place, 0, 1) };

                /// For a raw pointer to a struct, write a struct field without
                /// creating a reference to the field
                macro_rules! write_ptr_field {
                    ($target:ident, $field:ident, $value:expr) => {
                        ::core::ptr::write(::core::ptr::addr_of_mut!((*$target).$field), $value)
                    };
                }

                // SAFETY: try_ffi_init promises that `place` is writable
                unsafe {
                    // write_ptr_field!(place, ops, OperationsVtable::<T>::build());
                    write_ptr_field!(place, nr_hw_queues , nr_hw_queues);
                    write_ptr_field!(place, timeout , 0); // 0 means default which is 30 * HZ in C
                    write_ptr_field!(place, numa_node , bindings::NUMA_NO_NODE);
                    write_ptr_field!(place, queue_depth , num_tags);
                    write_ptr_field!(place, cmd_size , core::mem::size_of::<T::RequestData>().try_into()?);
                    write_ptr_field!(place, flags , bindings::BLK_MQ_F_SHOULD_MERGE);
                    write_ptr_field!(place, driver_data , tagset_data.into_foreign() as _);
                    write_ptr_field!(place, nr_maps , num_maps);
                }

                // SAFETY: Relevant fields of `place` are initialised above
                // let ret = unsafe { bindings::blk_mq_alloc_tag_set(place) };
                // if ret < 0 {
                //     // SAFETY: We created `driver_data` above with `into_foreign`
                //     unsafe { T::TagSetData::from_foreign((*place).driver_data) };
                //     return Err(Error::from_errno(ret));
                // }
                Ok(())
            }),
            _p: PhantomData,
            overwrite: false,
        }?Error);
        res
    }
    /// TODO Delete it
    /// Try to create a new tag set
    pub fn try_new<const IO: bool>(
        nr_hw_queues: u32,
        tagset_data: T::TagSetData,
        num_tags: u32,
        num_maps: u32,
        domain: T::DomainType,
    ) -> impl PinInit<Self, Error> {
        let domain_ptr = Box::into_raw(Box::new(domain));

        let tagset_data_shim = TagSetDataShim {
            original_tag_data: tagset_data.into_foreign() as _,
            domain: domain_ptr as _,
        };
        let tagset_data_shim_ptr = Box::into_raw(Box::new(tagset_data_shim)) as _;

        let res = try_pin_init!( TagSet {
            inner <- Opaque::try_ffi_init(move |place: *mut bindings::blk_mq_tag_set| -> Result<()> {

                // SAFETY: try_ffi_init promises that `place` is writable, and
                // zeroes is a valid bit pattern for this structure.
                unsafe { core::ptr::write_bytes(place, 0, 1) };

                /// For a raw pointer to a struct, write a struct field without
                /// creating a reference to the field
                macro_rules! write_ptr_field {
                    ($target:ident, $field:ident, $value:expr) => {
                        ::core::ptr::write(::core::ptr::addr_of_mut!((*$target).$field), $value)
                    };
                }

                // SAFETY: try_ffi_init promises that `place` is writable
                unsafe {
                    write_ptr_field!(place, ops, OperationsVtableShim::<IO,T>::build());
                    write_ptr_field!(place, nr_hw_queues , nr_hw_queues);
                    write_ptr_field!(place, timeout , 0); // 0 means default which is 30 * HZ in C
                    write_ptr_field!(place, numa_node , bindings::NUMA_NO_NODE);
                    write_ptr_field!(place, queue_depth , num_tags);
                    write_ptr_field!(place, cmd_size , size_of::<T::RequestData>().try_into()?);
                    write_ptr_field!(place, flags , bindings::BLK_MQ_F_SHOULD_MERGE);
                    // overwrite the driver_data field with the tagset_data_ptr
                    write_ptr_field!(place, driver_data , tagset_data_shim_ptr);
                    write_ptr_field!(place, nr_maps , num_maps);
                }

                // SAFETY: Relevant fields of `place` are initialised above
                let ret = crate::sys_blk_mq_alloc_tag_set(place) ;
                if ret < 0 {
                    // SAFETY: We created `driver_data` above with `into_foreign`
                    unsafe { T::TagSetData::from_foreign((*place).driver_data) };
                    return Err(Error::from_errno(ret));
                }
                Ok(())
            }),
            _p: PhantomData,
            overwrite: true,
        }?Error);
        res
    }
    /// Return the pointer to the wrapped `struct blk_mq_tag_set`
    pub fn raw_tag_set(&self) -> *mut bindings::blk_mq_tag_set {
        self.inner.get()
    }

    /// Create a `TagSet<T>` from a raw pointer.
    ///
    /// # Safety
    ///
    /// `ptr` must be a pointer to a valid and initialized `TagSet<T>`. There
    /// may be no other mutable references to the tag set. The pointee must be
    /// live and valid at least for the duration of the returned lifetime `'a`.
    pub(crate) unsafe fn from_ptr<'a>(ptr: *mut bindings::blk_mq_tag_set) -> &'a Self {
        // SAFETY: By the safety requirements of this function, `ptr` is valid
        // for use as a reference for the duration of `'a`.
        unsafe { &*(ptr.cast::<Self>()) }
    }

    pub fn tag_to_rq(&self, qid: u32, tag: u32) -> Option<RequestRef<'_, T>> {
        // TODO: We have to check that qid doesn't overflow hw queue.
        let tags = unsafe { *(*self.inner.get()).tags.add(qid as _) };
        let rq = crate::sys_blk_mq_tag_to_rq(tags, tag);
        if rq.is_null() {
            None
        } else {
            Some(unsafe { RequestRef::new(rq) })
        }
    }
}

#[pinned_drop]
impl<T: MqOperations> PinnedDrop for TagSet<T> {
    fn drop(self: Pin<&mut Self>) {
        // SAFETY: We are not moving self below
        let this = unsafe { Pin::into_inner_unchecked(self) };

        // SAFETY: `this.inner.get()` points to a valid `blk_mq_tag_set` and
        // thus is safe to dereference.
        let mut tagset_data_shim = unsafe { (*this.inner.get()).driver_data };

        if this.overwrite {
            let tagset_data = unsafe { Box::from_raw(tagset_data_shim as *mut TagSetDataShim) };
            tagset_data_shim = tagset_data.original_tag_data;
            // SAFETY: `inner` is valid and has been properly initialised during construction.
            crate::sys_blk_mq_free_tag_set(this.inner.get());

            // free domain
            let domain = unsafe { Box::from_raw(tagset_data.domain) };
            drop(domain);
        }

        // SAFETY: `tagset_data` was created by a call to
        // `ForeignOwnable::into_foreign` in `TagSet::try_new()`
        unsafe { T::TagSetData::from_foreign(tagset_data_shim) };
    }
}
