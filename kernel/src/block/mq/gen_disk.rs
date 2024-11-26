// SPDX-License-Identifier: GPL-2.0

//! GenDisk abstraction
//!
//! C header: [`include/linux/blkdev.h`](../../include/linux/blkdev.h)
//! C header: [`include/linux/blk_mq.h`](../../include/linux/blk_mq.h)

use alloc::sync::Arc;
use core::fmt::{self, Write};

use crate::{
    bindings,
    block::mq::{raw_writer::RawWriter, Operations, TagSet},
    error,
    error::{from_err_ptr, KernelResult as Result},
    pr_info,
    types::{ForeignOwnable, ScopeGuard},
};

/// A builder for [`GenDisk`].
///
/// Use this struct to configure and add new [`GenDisk`] to the VFS.
pub struct GenDiskBuilder {
    rotational: bool,
    logical_block_size: u32,
    physical_block_size: u32,
    capacity_sectors: u64,
    virt_boundary_mask: u64,
    max_segments: u16,
    max_hw_sectors: u32,
}
impl Default for GenDiskBuilder {
    fn default() -> Self {
        Self {
            rotational: false,
            logical_block_size: bindings::PAGE_SIZE as u32,
            physical_block_size: bindings::PAGE_SIZE as u32,
            capacity_sectors: 0,
            virt_boundary_mask: 0,
            max_segments: 0,
            max_hw_sectors: 0,
        }
    }
}

impl GenDiskBuilder {
    /// Create a new instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the rotational media attribute for the device to be built.
    pub fn rotational(mut self, rotational: bool) -> Self {
        self.rotational = rotational;
        self
    }

    /// Validate block size by verifying that it is between 512 and `PAGE_SIZE`,
    /// and that it is a power of two.
    fn validate_block_size(size: u32) -> Result<()> {
        if !(512..=bindings::PAGE_SIZE as u32).contains(&size) || !size.is_power_of_two() {
            Err(error::linux_err::EINVAL)
        } else {
            Ok(())
        }
    }

    /// Set the logical block size of the device to be built.
    ///
    /// This method will check that block size is a power of two and between 512
    /// and 4096. If not, an error is returned and the block size is not set.
    ///
    /// This is the smallest unit the storage device can address. It is
    /// typically 4096 bytes.
    pub fn logical_block_size(mut self, block_size: u32) -> Result<Self> {
        Self::validate_block_size(block_size)?;
        self.logical_block_size = block_size;
        Ok(self)
    }

    /// Set the physical block size of the device to be built.
    ///
    /// This method will check that block size is a power of two and between 512
    /// and 4096. If not, an error is returned and the block size is not set.
    ///
    /// This is the smallest unit a physical storage device can write
    /// atomically. It is usually the same as the logical block size but may be
    /// bigger. One example is SATA drives with 4096 byte physical block size
    /// that expose a 512 byte logical block size to the operating system.
    pub fn physical_block_size(mut self, block_size: u32) -> Result<Self> {
        Self::validate_block_size(block_size)?;
        self.physical_block_size = block_size;
        Ok(self)
    }

    /// Set the capacity of the device to be built, in sectors (512 bytes).
    pub fn capacity_sectors(mut self, capacity: u64) -> Self {
        self.capacity_sectors = capacity;
        self
    }

    pub fn virt_boundary_mask(mut self, boundary_mask: u64) -> Self {
        self.virt_boundary_mask = boundary_mask;
        self
    }

    pub fn max_segments(mut self, segments: u16) -> Self {
        self.max_segments = segments;
        self
    }

    pub fn max_hw_sectors(mut self, sectors: u32) -> Self {
        self.max_hw_sectors = sectors;
        self
    }

    /// Build a new `GenDisk` and add it to the VFS.
    pub fn build<T: Operations>(
        self,
        name: fmt::Arguments<'_>,
        tagset: Arc<TagSet<T>>,
        queue_data: T::QueueData,
    ) -> Result<GenDisk<T>> {
        let data = queue_data.into_foreign();
        let recover_data = ScopeGuard::new(|| {
            // SAFETY: T::QueueData was created by the call to `into_foreign()` above
            unsafe { T::QueueData::from_foreign(data) };
        });

        let lock_class_key = crate::sync::LockClassKey::new();

        // SAFETY: `bindings::queue_limits` contain only fields that are valid when zeroed.
        let mut lim: bindings::queue_limits = unsafe { core::mem::zeroed() };

        lim.logical_block_size = self.logical_block_size;
        lim.physical_block_size = self.physical_block_size;
        lim.max_hw_sectors = self.max_hw_sectors;
        lim.max_segments = self.max_segments;
        lim.virt_boundary_mask = self.virt_boundary_mask;

        // if self.rotational {
        //     lim.features = bindings::BLK_FEAT_ROTATIONAL;
        // }

        // SAFETY: `tagset.raw_tag_set()` points to a valid and initialized tag set
        let gendisk = from_err_ptr(unsafe {
            bindings::__blk_mq_alloc_disk(
                tagset.raw_tag_set(),
                // &mut lim,
                data.cast_mut(),
                lock_class_key.as_ptr(),
            )
        })?;

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
            // TODO: Set to THIS_MODULE. Waiting for const_refs_to_static feature to
            // be merged (unstable in rustc 1.78 which is staged for linux 6.10)
            // https://github.com/rust-lang/rust/issues/119618
            owner: core::ptr::null_mut(),
            pr_ops: core::ptr::null_mut(),
            free_disk: None,
            poll_bio: None,
        };

        // SAFETY: `gendisk` is a valid pointer as we initialized it above
        unsafe { (*gendisk).fops = &TABLE };

        let mut raw_writer = RawWriter::from_array(
            // SAFETY: `gendisk` points to a valid and initialized instance. We
            // have exclusive access, since the disk is not added to the VFS
            // yet.
            unsafe { &mut (*gendisk).disk_name },
        )?;
        raw_writer.write_fmt(name)?;
        raw_writer.write_char('\0')?;

        // SAFETY: `gendisk` points to a valid and initialized instance of
        // `struct gendisk`. `set_capacity` takes a lock to synchronize this
        // operation, so we will not race.
        unsafe { bindings::set_capacity(gendisk, self.capacity_sectors) };

        error::to_result(
            // SAFETY: `gendisk` points to a valid and initialized instance of
            // `struct gendisk`.
            unsafe {
                bindings::device_add_disk(core::ptr::null_mut(), gendisk, core::ptr::null_mut())
            },
        )?;

        recover_data.dismiss();

        let gendisk = GenDisk {
            _tagset: tagset,
            gendisk,
        };
        // lim.logical_block_size = self.logical_block_size;
        // lim.physical_block_size = self.physical_block_size;
        // lim.max_hw_sectors = self.max_hw_sectors;
        // lim.max_segments = self.max_segments;
        // lim.virt_boundary_mask = self.virt_boundary_mask;

        gendisk.set_queue_logical_block_size(self.logical_block_size);
        gendisk.set_queue_physical_block_size(self.physical_block_size);
        gendisk.set_queue_virt_boundary(self.virt_boundary_mask as usize);
        gendisk.set_queue_max_hw_sectors(self.max_hw_sectors);
        gendisk.set_queue_max_segments(self.max_segments);

        // INVARIANT: `gendisk` was initialized above.
        // INVARIANT: `gendisk` was added to the VFS via `device_add_disk` above.
        // INVARIANT: `gendisk.queue.queue_data` is set to `data` in the call to
        // `__blk_mq_alloc_disk` above.
        Ok(gendisk)
    }
}
/// A generic block device
///
/// # Invariants
///
///  - `gendisk` must always point to an initialized and valid `struct gendisk`.
pub struct GenDisk<T: Operations> {
    _tagset: Arc<TagSet<T>>,
    gendisk: *mut bindings::gendisk,
}

// SAFETY: `GenDisk` is an owned pointer to a `struct gendisk` and an `Arc` to a
// `TagSet` It is safe to send this to other threads as long as T is Send.
unsafe impl<T: Operations + Send> Send for GenDisk<T> {}

impl<T: Operations> GenDisk<T> {
    /// Try to create a new `GenDisk`
    pub fn try_new(tagset: Arc<TagSet<T>>, queue_data: T::QueueData) -> Result<Self> {
        let data = queue_data.into_foreign();
        let recover_data = ScopeGuard::new(|| {
            // SAFETY: T::QueueData was created by the call to `into_foreign()` above
            unsafe { T::QueueData::from_foreign(data) };
        });

        let lock_class_key = crate::sync::LockClassKey::new();
        // SAFETY: `tagset.raw_tag_set()` points to a valid and initialized tag set
        let gendisk = from_err_ptr(unsafe {
            bindings::__blk_mq_alloc_disk(tagset.raw_tag_set(), data as _, lock_class_key.as_ptr())
        })?;
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
            _tagset: tagset,
            gendisk,
        })
    }

    /// Set the name of the device
    pub fn set_name(&self, args: fmt::Arguments<'_>) -> Result {
        let mut raw_writer = RawWriter::from_array(unsafe { &mut (*self.gendisk).disk_name })?;
        raw_writer.write_fmt(args)?;
        raw_writer.write_char('\0')?;
        Ok(())
    }

    /// Register the device with the kernel. When this function return, the
    /// device is accessible from VFS. The kernel may issue reads to the device
    /// during registration to discover partition infomation.
    pub fn add(&self) -> Result {
        pr_info!("before device_add_disk");
        let res = crate::error::to_result(unsafe {
            bindings::device_add_disk(core::ptr::null_mut(), self.gendisk, core::ptr::null_mut())
        });
        pr_info!("after device_add_disk");
        res
    }

    /// Call to tell the block layer the capcacity of the device
    pub fn set_capacity(&self, sectors: u64) {
        unsafe { bindings::set_capacity(self.gendisk, sectors) };
    }

    /// Set the logical block size of the device
    pub fn set_queue_logical_block_size(&self, size: u32) {
        unsafe { bindings::blk_queue_logical_block_size((*self.gendisk).queue, size) };
    }

    /// Set the physical block size of the device
    pub fn set_queue_physical_block_size(&self, size: u32) {
        unsafe { bindings::blk_queue_physical_block_size((*self.gendisk).queue, size) };
    }

    pub fn set_queue_virt_boundary(&self, mask: usize) {
        unsafe { bindings::blk_queue_virt_boundary((*self.gendisk).queue, mask as _) };
    }

    pub fn set_queue_max_hw_sectors(&self, max_hw_sectors: u32) {
        unsafe { bindings::blk_queue_max_hw_sectors((*self.gendisk).queue, max_hw_sectors) };
    }

    pub fn set_queue_max_segments(&self, max_segments: u16) {
        unsafe { bindings::blk_queue_max_segments((*self.gendisk).queue, max_segments) };
    }

    /// Set the rotational media attribute for the device
    pub fn set_rotational(&self, rotational: bool) {
        if !rotational {
            unsafe {
                bindings::blk_queue_flag_set(bindings::QUEUE_FLAG_NONROT, (*self.gendisk).queue)
            };
        } else {
            unsafe {
                bindings::blk_queue_flag_clear(bindings::QUEUE_FLAG_NONROT, (*self.gendisk).queue)
            };
        }
    }
}

impl<T: Operations> Drop for GenDisk<T> {
    fn drop(&mut self) {
        let queue_data = unsafe { (*(*self.gendisk).queue).queuedata };
        unsafe { bindings::del_gendisk(self.gendisk) };
        // SAFETY: `queue.queuedata` was created by `GenDisk::try_new()` with a
        // call to `ForeignOwnable::into_pointer()` to create `queuedata`.
        // `ForeignOwnable::from_foreign()` is only called here.
        let _queue_data = unsafe { T::QueueData::from_foreign(queue_data) };
    }
}
