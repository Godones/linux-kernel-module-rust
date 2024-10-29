use downcast_rs::{impl_downcast, DowncastSync};
use kbind::safe_ptr::SafePtr;

use crate::{Basic, LinuxResult};

pub trait BlockDeviceDomain: Basic + DowncastSync {
    fn init(&self, args: &BlockArgs) -> LinuxResult<()>;
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

impl_downcast!(sync BlockDeviceDomain);

#[derive(Debug, Copy, Clone)]
pub struct BlockArgs {
    // Use memory backing
    pub param_memory_backed: bool,
    // IRQ Mode (0: None, 1: Soft, 2: Timer)
    pub param_irq_mode: u8,
    // Device capacity in MiB
    pub param_capacity_mib: u64,
    // Completion time in nano seconds for timer mode
    pub param_completion_time_nsec: u64,
}

impl Default for BlockArgs {
    fn default() -> Self {
        Self {
            param_memory_backed: true,
            param_irq_mode: 0,
            param_capacity_mib: 4096,
            param_completion_time_nsec: 0,
        }
    }
}
