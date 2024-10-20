use downcast_rs::{impl_downcast, DowncastSync};

use crate::{Basic, LinuxResult};

pub trait BlockDeviceDomain: Basic + DowncastSync {
    fn init(&self, args: &BlockArgs) -> LinuxResult<()>;
    fn tag_set_with_queue_data(&self) -> LinuxResult<(usize, usize)>;

    /// Domain should set the gendisk parameter
    fn set_gen_disk(&self, gen_disk: usize) -> LinuxResult<usize>;

    /// Open the block device
    fn open(&self, mode: u32) -> LinuxResult<()>;
    //// Release the block device
    fn release(&self) -> LinuxResult<()>;

    //tagset
    fn init_request(
        &self,
        tag_set_ptr: usize,
        rq_ptr: usize,
        driver_data_ptr: usize,
    ) -> LinuxResult<()>;
    fn exit_request(&self, tag_set_ptr: usize, rq_ptr: usize) -> LinuxResult<()>;
    fn init_hctx(
        &self,
        hctx_ptr: usize,
        tag_set_data_ptr: usize,
        hctx_idx: usize,
    ) -> LinuxResult<()>;
    fn exit_hctx(&self, hctx_ptr: usize, hctx_idx: usize) -> LinuxResult<()>;

    fn queue_rq(
        &self,
        hctx_ptr: usize,
        bd_ptr: usize,
        hctx_driver_data_ptr: usize,
    ) -> LinuxResult<()>;
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
            param_completion_time_nsec: 1_000_000,
        }
    }
}
