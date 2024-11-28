use alloc::boxed::Box;
use core::{
    fmt::{Debug, Formatter},
    pin::Pin,
};

use basic::{
    c_str,
    console::{print as pr_info, println},
    kernel::{
        block::mq::OperationsConverter,
        driver::DriverRegistration,
        irq::IrqHandlerShim,
        pci::{PciAdapter, PciAdapterShim},
        ThisModule,
    },
    static_assert, LinuxError, LinuxResult, SafePtr,
};
use interface::{
    nvme::{BlkMqOp, IrqHandlerOp, NvmeBlockArgs, NvmeBlockDeviceDomain, PCIDeviceOp},
    Basic,
};
use spin::Mutex;

use crate::{
    nvme_mq::{AdminQueueOperations, IoQueueOperations},
    nvme_queue::NvmeQueueIrqHandler,
    MappingData, NvmeDevice, NVME_IRQ_QUEUE_COUNT, NVME_POLL_QUEUE_COUNT,
};

pub struct NvmeDomain {
    _registration: Mutex<Option<Pin<Box<DriverRegistration<PciAdapter<NvmeDevice>>>>>>,
}

unsafe impl Send for NvmeDomain {}
unsafe impl Sync for NvmeDomain {}

impl Basic for NvmeDomain {
    fn domain_id(&self) -> u64 {
        rref::domain_id()
    }
}

impl Debug for NvmeDomain {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "NvmeDomain")
    }
}

impl IrqHandlerOp for NvmeDomain {
    fn handle_irq(&self, data: SafePtr) -> LinuxResult<u32> {
        let res =
            IrqHandlerShim::<NvmeQueueIrqHandler>::handle_irq(data).expect("irq handler failed");
        Ok(res)
    }
}

impl PCIDeviceOp for NvmeDomain {
    fn probe(&self, pdev: SafePtr, pci_device_id: SafePtr) -> LinuxResult<i32> {
        PciAdapterShim::<NvmeDevice>::probe(pdev, pci_device_id).expect("probe failed");
        Ok(0)
    }

    fn remove(&self, pdev: SafePtr) -> LinuxResult<()> {
        PciAdapterShim::<NvmeDevice>::remove(pdev).expect("remove failed");
        Ok(())
    }
}

impl BlkMqOp for NvmeDomain {
    fn init_request(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        let res = if io_queue {
            OperationsConverter::<IoQueueOperations>::init_request(
                tag_set_ptr,
                rq_ptr,
                driver_data_ptr,
            )
        } else {
            OperationsConverter::<AdminQueueOperations>::init_request(
                tag_set_ptr,
                rq_ptr,
                driver_data_ptr,
            )
        };
        res.map_err(|e| {
            println!("NullBlkModule init_request error: {:?}", e);
            LinuxError::EINVAL
        })
    }

    fn exit_request(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        let res = if io_queue {
            OperationsConverter::<IoQueueOperations>::exit_request(tag_set_ptr, rq_ptr)
        } else {
            OperationsConverter::<AdminQueueOperations>::exit_request(tag_set_ptr, rq_ptr)
        };
        res.map_err(|e| {
            println!("NullBlkModule exit_request error: {:?}", e);
            LinuxError::EINVAL
        })
    }

    fn init_hctx(
        &self,
        hctx_ptr: SafePtr,
        tag_set_data_ptr: SafePtr,
        hctx_idx: usize,
        io_queue: bool,
    ) -> LinuxResult<()> {
        let res = if io_queue {
            OperationsConverter::<IoQueueOperations>::init_hctx(
                hctx_ptr,
                tag_set_data_ptr,
                hctx_idx,
            )
        } else {
            OperationsConverter::<AdminQueueOperations>::init_hctx(
                hctx_ptr,
                tag_set_data_ptr,
                hctx_idx,
            )
        };
        res.map_err(|e| {
            println!("NullBlkModule init_hctx error: {:?}", e);
            LinuxError::EINVAL
        })
    }

    fn exit_hctx(&self, hctx_ptr: SafePtr, hctx_idx: usize, io_queue: bool) -> LinuxResult<()> {
        let res = if io_queue {
            OperationsConverter::<IoQueueOperations>::exit_hctx(hctx_ptr, hctx_idx)
        } else {
            OperationsConverter::<AdminQueueOperations>::exit_hctx(hctx_ptr, hctx_idx)
        };
        res.map_err(|e| {
            println!("NullBlkModule exit_hctx error: {:?}", e);
            LinuxError::EINVAL
        })
    }

    fn queue_rq(
        &self,
        hctx_ptr: SafePtr,
        bd_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        let res = if io_queue {
            OperationsConverter::<IoQueueOperations>::queue_rq(
                hctx_ptr,
                bd_ptr,
                hctx_driver_data_ptr,
            )
        } else {
            OperationsConverter::<AdminQueueOperations>::queue_rq(
                hctx_ptr,
                bd_ptr,
                hctx_driver_data_ptr,
            )
        };
        res.map_err(|e| {
            println!("NullBlkModule queue_rq error: {:?}", e);
            LinuxError::EINVAL
        })
    }

    fn commit_rqs(
        &self,
        hctx_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        let res = if io_queue {
            OperationsConverter::<IoQueueOperations>::commit_rqs(hctx_ptr, hctx_driver_data_ptr)
        } else {
            OperationsConverter::<AdminQueueOperations>::commit_rqs(hctx_ptr, hctx_driver_data_ptr)
        };
        res.map_err(|e| {
            println!("NullBlkModule commit_rqs error: {:?}", e);
            LinuxError::EINVAL
        })
    }

    fn complete_request(&self, rq_ptr: SafePtr, io_queue: bool) -> LinuxResult<()> {
        let res = if io_queue {
            OperationsConverter::<IoQueueOperations>::complete_request(rq_ptr)
        } else {
            OperationsConverter::<AdminQueueOperations>::complete_request(rq_ptr)
        };
        res.map_err(|e| {
            println!("NullBlkModule complete_request error: {:?}", e);
            LinuxError::EINVAL
        })
    }

    fn map_queues(
        &self,
        tag_set_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        let res = if io_queue {
            OperationsConverter::<IoQueueOperations>::map_queues(tag_set_ptr, driver_data_ptr)
        } else {
            OperationsConverter::<AdminQueueOperations>::map_queues(tag_set_ptr, driver_data_ptr)
        };
        res.map_err(|e| {
            println!("NullBlkModule map_queues error: {:?}", e);
            LinuxError::EINVAL
        })
    }

    fn poll_queues(
        &self,
        hctx_ptr: SafePtr,
        iob_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<i32> {
        let res = if io_queue {
            OperationsConverter::<IoQueueOperations>::poll_queues(
                hctx_ptr,
                iob_ptr,
                hctx_driver_data_ptr,
            )
        } else {
            OperationsConverter::<AdminQueueOperations>::poll_queues(
                hctx_ptr,
                iob_ptr,
                hctx_driver_data_ptr,
            )
        };
        res.map_err(|e| {
            println!("NullBlkModule poll_queues error: {:?}", e);
            LinuxError::EINVAL
        })
    }
}

impl NvmeDomain {
    pub fn new() -> Self {
        Self {
            _registration: Mutex::new(None),
        }
    }
}

impl NvmeBlockDeviceDomain for NvmeDomain {
    fn init(&self, args: &NvmeBlockArgs) -> LinuxResult<()> {
        pr_info!("Nvme module loaded!\n");
        static_assert!(size_of::<MappingData>() <= basic::bindings::PAGE_SIZE as usize);
        let module = ThisModule::from_safe_ptr(args.module);

        NVME_IRQ_QUEUE_COUNT.call_once(|| args.nvme_irq_queue_count);
        NVME_POLL_QUEUE_COUNT.call_once(|| args.nvme_poll_queue_count);
        super::DOMAIN_SELF.lock().replace(args.nvme_domain.clone());

        let registration =
            DriverRegistration::new_pinned(c_str!("nvme"), module, args.nvme_domain.clone() as _)
                .map_err(|_| LinuxError::EINVAL)?;
        pr_info!("pci driver registered\n");
        self._registration.lock().replace(registration);
        Ok(())
    }

    fn exit(&self) -> LinuxResult<()> {
        super::DOMAIN_SELF.lock().take();
        self._registration.lock().take();
        Ok(())
    }
}

#[derive(Debug)]
pub struct UnwindWrap(NvmeDomain);

impl UnwindWrap {
    pub fn new(nvme: NvmeDomain) -> Self {
        Self(nvme)
    }
}

impl Basic for UnwindWrap {
    fn domain_id(&self) -> u64 {
        self.0.domain_id()
    }
}

impl IrqHandlerOp for UnwindWrap {
    fn handle_irq(&self, data: SafePtr) -> LinuxResult<u32> {
        basic::catch_unwind(|| self.0.handle_irq(data))
    }
}

impl PCIDeviceOp for UnwindWrap {
    fn probe(&self, pdev: SafePtr, pci_device_id: SafePtr) -> LinuxResult<i32> {
        basic::catch_unwind(|| self.0.probe(pdev, pci_device_id))
    }

    fn remove(&self, pdev: SafePtr) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.remove(pdev))
    }
}

impl BlkMqOp for UnwindWrap {
    fn init_request(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        basic::catch_unwind(|| {
            self.0
                .init_request(tag_set_ptr, rq_ptr, driver_data_ptr, io_queue)
        })
    }

    fn exit_request(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.exit_request(tag_set_ptr, rq_ptr, io_queue))
    }

    fn init_hctx(
        &self,
        hctx_ptr: SafePtr,
        tag_set_data_ptr: SafePtr,
        hctx_idx: usize,
        io_queue: bool,
    ) -> LinuxResult<()> {
        basic::catch_unwind(|| {
            self.0
                .init_hctx(hctx_ptr, tag_set_data_ptr, hctx_idx, io_queue)
        })
    }

    fn exit_hctx(&self, hctx_ptr: SafePtr, hctx_idx: usize, io_queue: bool) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.exit_hctx(hctx_ptr, hctx_idx, io_queue))
    }

    fn queue_rq(
        &self,
        hctx_ptr: SafePtr,
        bd_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        basic::catch_unwind(|| {
            self.0
                .queue_rq(hctx_ptr, bd_ptr, hctx_driver_data_ptr, io_queue)
        })
    }

    fn commit_rqs(
        &self,
        hctx_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.commit_rqs(hctx_ptr, hctx_driver_data_ptr, io_queue))
    }

    fn complete_request(&self, rq_ptr: SafePtr, io_queue: bool) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.complete_request(rq_ptr, io_queue))
    }

    fn map_queues(
        &self,
        tag_set_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.map_queues(tag_set_ptr, driver_data_ptr, io_queue))
    }

    fn poll_queues(
        &self,
        hctx_ptr: SafePtr,
        iob_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<i32> {
        basic::catch_unwind(|| {
            self.0
                .poll_queues(hctx_ptr, iob_ptr, hctx_driver_data_ptr, io_queue)
        })
    }
}

impl NvmeBlockDeviceDomain for UnwindWrap {
    fn init(&self, args: &NvmeBlockArgs) -> LinuxResult<()> {
        self.0.init(args)
    }
    fn exit(&self) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.exit())
    }
}
