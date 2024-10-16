use downcast_rs::{impl_downcast, DowncastSync};

use crate::{Basic, LinuxResult};

pub trait BlockDeviceDomain: Basic + DowncastSync {
    fn init(&self, args: &BlockArgs) -> LinuxResult<()>;
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
