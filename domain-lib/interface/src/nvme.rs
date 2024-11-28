use alloc::sync::Arc;

use downcast_rs::{impl_downcast, DowncastSync};
use kbind::safe_ptr::SafePtr;

use crate::{Basic, LinuxResult};

pub trait PCIDeviceOp: DowncastSync {
    fn probe(&self, pdev: SafePtr, pci_device_id: SafePtr) -> LinuxResult<i32>;
    fn remove(&self, pdev: SafePtr) -> LinuxResult<()>;
}

pub trait IrqHandlerOp: DowncastSync {
    fn handle_irq(&self, data: SafePtr) -> LinuxResult<u32>;
}

pub trait BlkMqOp: DowncastSync {
    fn init_request(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()>;
    fn exit_request(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()>;
    fn init_hctx(
        &self,
        hctx_ptr: SafePtr,
        tag_set_data_ptr: SafePtr,
        hctx_idx: usize,
        io_queue: bool,
    ) -> LinuxResult<()>;
    fn exit_hctx(&self, hctx_ptr: SafePtr, hctx_idx: usize, io_queue: bool) -> LinuxResult<()>;
    fn queue_rq(
        &self,
        hctx_ptr: SafePtr,
        bd_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()>;
    fn commit_rqs(
        &self,
        hctx_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()>;
    fn complete_request(&self, rq_ptr: SafePtr, io_queue: bool) -> LinuxResult<()>;
    fn map_queues(
        &self,
        tag_set_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()>;
    fn poll_queues(
        &self,
        hctx_ptr: SafePtr,
        iob_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<i32>;
}

pub trait NvmeBlockDeviceDomain:
    IrqHandlerOp + PCIDeviceOp + BlkMqOp + Basic + DowncastSync
{
    fn init(&self, args: &NvmeBlockArgs) -> LinuxResult<()>;
    fn exit(&self) -> LinuxResult<()>;
}

impl_downcast!(sync NvmeBlockDeviceDomain);

#[derive(Debug, Clone)]
pub struct NvmeBlockArgs {
    pub nvme_irq_queue_count: i64,
    pub nvme_poll_queue_count: i64,
    pub nvme_domain: Arc<dyn NvmeBlockDeviceDomain>,
    pub module: SafePtr,
}

impl NvmeBlockArgs {
    pub fn new(
        irq_queue_count: Option<i64>,
        poll_queue_count: Option<i64>,
        nvme_domain: Arc<dyn NvmeBlockDeviceDomain>,
        module: SafePtr,
    ) -> Self {
        Self {
            nvme_irq_queue_count: irq_queue_count.unwrap_or(1),
            nvme_poll_queue_count: poll_queue_count.unwrap_or(1),
            nvme_domain,
            module,
        }
    }
}
