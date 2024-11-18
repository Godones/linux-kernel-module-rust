// SPDX-License-Identifier: GPL-2.0

//! GenDisk abstraction
//!
//! C header: [`include/linux/blkdev.h`](../../include/linux/blkdev.h)
//! C header: [`include/linux/blk_mq.h`](../../include/linux/blk_mq.h)

use alloc::sync::Arc;
use core::{
    ffi::c_void,
    fmt::{self, Write},
};

use kbind::safe_ptr::SafePtr;

use crate::{
    bindings,
    kernel::{
        block::mq::{raw_writer::RawWriter, MqOperations, TagSet},
        error,
        error::{from_err_ptr, KernelResult as Result},
        sync::LockClassKey,
        types::{ForeignOwnable, ScopeGuard},
    },
};

/// A generic block device
///
/// # Invariants
///
///  - `gendisk` must always point to an initialized and valid `struct gendisk`.
pub struct GenDisk<T: MqOperations> {
    tagset: Arc<TagSet<T>>,
    gendisk: *mut bindings::gendisk,
    queue_data: *const c_void,
    over_write: bool,
}

// SAFETY: `GenDisk` is an owned pointer to a `struct gendisk` and an `Arc` to a
// `TagSet` It is safe to send this to other threads as long as T is Send.
unsafe impl<T: MqOperations + Send> Send for GenDisk<T> {}

impl<T: MqOperations> GenDisk<T> {
    pub fn new_no_alloc(tagset: Arc<TagSet<T>>, queue_data: T::QueueData) -> Self {
        Self {
            tagset,
            gendisk: core::ptr::null_mut(),
            queue_data: queue_data.into_foreign(),
            over_write: false,
        }
    }

    pub fn set_gen_disk(&mut self, gendisk: SafePtr) {
        unsafe {
            self.gendisk = gendisk.raw_ptr() as *mut bindings::gendisk;
        }
    }

    pub fn tagset_ptr(&self) -> SafePtr {
        unsafe { SafePtr::new(self.tagset.raw_tag_set() as _) }
    }

    pub fn queue_data_ptr(&self) -> SafePtr {
        unsafe { SafePtr::new(self.queue_data as *mut c_void) }
    }

    /// Try to create a new `GenDisk`
    pub fn try_new(tagset: Arc<TagSet<T>>, queue_data: T::QueueData) -> Result<Self> {
        let data = queue_data.into_foreign();
        let recover_data = ScopeGuard::new(|| {
            // SAFETY: T::QueueData was created by the call to `into_foreign()` above
            unsafe { T::QueueData::from_foreign(data) };
        });

        let lock_class_key = LockClassKey::new();

        // SAFETY: `tagset.raw_tag_set()` points to a valid and initialized tag set
        let gendisk = from_err_ptr(crate::sys__blk_mq_alloc_disk(
            tagset.raw_tag_set(),
            data as _,
            lock_class_key.as_ptr(),
        ))?;

        const TABLE: bindings::block_device_operations = bindings::block_device_operations {
            submit_bio: None,
            open: None,
            release: None,
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

        // SAFETY: gendisk is a valid pointer as we initialized it above
        unsafe { (*gendisk).fops = &TABLE };

        recover_data.dismiss();
        Ok(Self {
            tagset,
            gendisk,
            queue_data: data,
            over_write: true,
        })
    }

    /// Set the name of the device
    pub fn set_name(&self, args: fmt::Arguments<'_>) -> Result {
        let mut raw_writer = RawWriter::from_array(unsafe { &mut (*self.gendisk).disk_name });
        raw_writer.write_fmt(args)?;
        raw_writer.write_char('\0')?;
        Ok(())
    }

    /// Register the device with the kernel. When this function return, the
    /// device is accessible from VFS. The kernel may issue reads to the device
    /// during registration to discover partition infomation.
    pub fn add(&self) -> Result {
        error::to_result(crate::sys_device_add_disk(
            core::ptr::null_mut(),
            self.gendisk,
            core::ptr::null_mut(),
        ))
    }

    /// Call to tell the block layer the capcacity of the device
    pub fn set_capacity(&self, sectors: u64) {
        crate::sys_set_capacity(self.gendisk, sectors);
    }

    /// Set the logical block size of the device
    pub fn set_queue_logical_block_size(&self, size: u32) {
        unsafe { crate::sys_blk_queue_logical_block_size((*self.gendisk).queue, size) };
    }

    /// Set the physical block size of the device
    pub fn set_queue_physical_block_size(&self, size: u32) {
        unsafe { crate::sys_blk_queue_physical_block_size((*self.gendisk).queue, size) };
    }

    pub fn set_queue_virt_boundary(&self, mask: usize) {
        unsafe { crate::sys_blk_queue_virt_boundary((*self.gendisk).queue, mask as _) };
    }

    pub fn set_queue_max_hw_sectors(&self, max_hw_sectors: u32) {
        unsafe { crate::sys_blk_queue_max_hw_sectors((*self.gendisk).queue, max_hw_sectors) };
    }

    pub fn set_queue_max_segments(&self, max_segments: u16) {
        unsafe { crate::sys_blk_queue_max_segments((*self.gendisk).queue, max_segments) };
    }
    /// Set the rotational media attribute for the device
    pub fn set_rotational(&self, rotational: bool) {
        if !rotational {
            unsafe {
                crate::sys_blk_queue_flag_set(bindings::QUEUE_FLAG_NONROT, (*self.gendisk).queue)
            };
        } else {
            unsafe {
                crate::sys_blk_queue_flag_clear(bindings::QUEUE_FLAG_NONROT, (*self.gendisk).queue)
            };
        }
    }
}

impl<T: MqOperations> Drop for GenDisk<T> {
    fn drop(&mut self) {
        let queue_data = unsafe { (*(*self.gendisk).queue).queuedata };

        if self.over_write {
            crate::sys_del_gendisk(self.gendisk);
        }

        // SAFETY: `queue.queuedata` was created by `GenDisk::try_new()` with a
        // call to `ForeignOwnable::into_pointer()` to create `queuedata`.
        // `ForeignOwnable::from_foreign()` is only called here.
        let _queue_data = unsafe { T::QueueData::from_foreign(queue_data) };
    }
}
