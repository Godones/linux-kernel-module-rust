use downcast_rs::{impl_downcast, DowncastSync};
use kbind::safe_ptr::SafePtr;

use crate::{Basic, LinuxResult};

pub trait NvmeBlockDeviceDomain: Basic + DowncastSync {
    fn init(&self, args: &NvmeBlockArgs) -> LinuxResult<()>;
    fn tag_set_with_queue_data(&self) -> LinuxResult<(SafePtr, SafePtr)>;

    /// Domain should set the gendisk parameter
    fn set_gen_disk(&self, gen_disk: SafePtr) -> LinuxResult<()>;
    /// Open the block device
    fn open(&self, mode: u32) -> LinuxResult<()>;
    /// Release the block device
    fn release(&self) -> LinuxResult<()>;
    // tagset
    fn init_request(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        driver_data_ptr: SafePtr,
    ) -> LinuxResult<()>;
    fn exit_request(&self, tag_set_ptr: SafePtr, rq_ptr: SafePtr) -> LinuxResult<()>;
    fn init_hctx(
        &self,
        hctx_ptr: SafePtr,
        tag_set_data_ptr: SafePtr,
        hctx_idx: usize,
    ) -> LinuxResult<()>;
    fn exit_hctx(&self, hctx_ptr: SafePtr, hctx_idx: usize) -> LinuxResult<()>;
    fn queue_rq(
        &self,
        hctx_ptr: SafePtr,
        bd_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
    ) -> LinuxResult<()>;
    fn commit_rqs(&self, hctx_ptr: SafePtr, hctx_driver_data_ptr: SafePtr) -> LinuxResult<()>;
    fn complete_request(&self, rq_ptr: SafePtr) -> LinuxResult<()>;
    fn exit(&self) -> LinuxResult<()>;
}

impl_downcast!(sync NvmeBlockDeviceDomain);

#[derive(Debug, Copy, Clone)]
pub struct NvmeBlockArgs {
    nvme_irq_queue_count: i64,
    nvme_poll_queue_count: i64,
}

impl Default for NvmeBlockArgs {
    fn default() -> Self {
        Self {
            nvme_irq_queue_count: 1,
            nvme_poll_queue_count: 1,
        }
    }
}
