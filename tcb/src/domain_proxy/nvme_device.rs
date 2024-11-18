use alloc::boxed::Box;
use core::{any::Any, mem::forget, pin::Pin, sync::atomic::AtomicBool};

use basic::SafePtr;
use corelib::{LinuxError, LinuxResult};
use interface::{
    nvme::{BlkMqOp, IrqHandlerOp, NvmeBlockArgs, NvmeBlockDeviceDomain, PCIDeviceOp},
    Basic,
};
use kernel::{
    init::InPlaceInit,
    sync::{LongLongPerCpu, Mutex, SRcuData},
};
use spin::Once;

use crate::{
    domain_helper::{free_domain_resource, FreeShared},
    domain_loader::loader::DomainLoader,
    domain_proxy::ProxyBuilder,
};

#[derive(Debug)]
pub struct NvmeDeviceDomainProxy {
    domain: SRcuData<Box<dyn NvmeBlockDeviceDomain>>,
    lock: Pin<Box<Mutex<()>>>,
    domain_loader: Pin<Box<Mutex<DomainLoader>>>,
    flag: AtomicBool,
    counter: LongLongPerCpu,
    resource: Once<Box<dyn Any + Send + Sync>>,
}

impl NvmeDeviceDomainProxy {
    pub fn new(domain: Box<dyn NvmeBlockDeviceDomain>, domain_loader: DomainLoader) -> Self {
        NvmeDeviceDomainProxy {
            domain: SRcuData::new(domain),
            lock: Box::pin_init(new_mutex!(())).unwrap(),
            domain_loader: Box::pin_init(new_mutex!(domain_loader)).unwrap(),
            flag: AtomicBool::new(false),
            counter: LongLongPerCpu::new(),
            resource: Once::new(),
        }
    }
}
impl ProxyBuilder for NvmeDeviceDomainProxy {
    type T = Box<dyn NvmeBlockDeviceDomain>;

    fn build(domain: Self::T, domain_loader: DomainLoader) -> Self {
        Self::new(domain, domain_loader)
    }

    fn build_empty(domain_loader: DomainLoader) -> Self {
        Self::new(Box::new(NvmeDeviceDomainEmptyImpl::new()), domain_loader)
    }
    fn build_empty_no_proxy() -> Self::T {
        Box::new(NvmeDeviceDomainEmptyImpl::new())
    }

    fn init_by_box(&self, argv: Box<dyn Any + Send + Sync>) -> LinuxResult<()> {
        let args = argv
            .downcast_ref::<NvmeBlockArgs>()
            .ok_or(LinuxError::EINVAL)?;
        self.init(args)?;
        self.resource.call_once(|| argv);
        Ok(())
    }
}

impl NvmeDeviceDomainProxy {
    #[inline]
    fn _exit(&self) -> LinuxResult<()> {
        self.domain.read_directly(|domain| domain.exit())
    }
    #[inline]
    fn _exit_no_lock(&self) -> LinuxResult<()> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._exit();
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }

    #[inline]
    fn _exit_with_lock(&self) -> LinuxResult<()> {
        let lock = self.lock.lock();
        let r = self._exit();
        drop(lock);
        r
    }
    #[inline]
    fn _domain_id(&self) -> u64 {
        self.domain.read_directly(|domain| domain.domain_id())
    }
    #[inline]
    fn _domain_id_no_lock(&self) -> u64 {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._domain_id();
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }
    #[inline]
    fn _domain_id_with_lock(&self) -> u64 {
        let lock = self.lock.lock();
        let r = self._domain_id();
        drop(lock);
        r
    }

    #[inline]
    fn _init_request(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        self.domain.read_directly(|domain| {
            domain.init_request(tag_set_ptr, rq_ptr, driver_data_ptr, io_queue)
        })
    }
    #[inline]
    fn _init_request_no_lock(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._init_request(tag_set_ptr, rq_ptr, driver_data_ptr, io_queue);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }
    #[inline]
    fn _init_request_with_lock(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        let lock = self.lock.lock();
        let r = self._init_request(tag_set_ptr, rq_ptr, driver_data_ptr, io_queue);
        drop(lock);
        r
    }

    #[inline]
    fn _exit_request(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        self.domain
            .read_directly(|domain| domain.exit_request(tag_set_ptr, rq_ptr, io_queue))
    }
    #[inline]
    fn _exit_request_no_lock(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._exit_request(tag_set_ptr, rq_ptr, io_queue);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }
    #[inline]
    fn _exit_request_with_lock(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        let lock = self.lock.lock();
        let r = self._exit_request(tag_set_ptr, rq_ptr, io_queue);
        drop(lock);
        r
    }

    #[inline]
    fn _init_hctx(
        &self,
        hctx_ptr: SafePtr,
        tag_set_data_ptr: SafePtr,
        hctx_idx: usize,
        io_queue: bool,
    ) -> LinuxResult<()> {
        self.domain.read_directly(|domain| {
            domain.init_hctx(hctx_ptr, tag_set_data_ptr, hctx_idx, io_queue)
        })
    }
    #[inline]
    fn _init_hctx_no_lock(
        &self,
        hctx_ptr: SafePtr,
        tag_set_data_ptr: SafePtr,
        hctx_idx: usize,
        io_queue: bool,
    ) -> LinuxResult<()> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._init_hctx(hctx_ptr, tag_set_data_ptr, hctx_idx, io_queue);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }
    #[inline]
    fn _init_hctx_with_lock(
        &self,
        hctx_ptr: SafePtr,
        tag_set_data_ptr: SafePtr,
        hctx_idx: usize,
        io_queue: bool,
    ) -> LinuxResult<()> {
        let lock = self.lock.lock();
        let r = self._init_hctx(hctx_ptr, tag_set_data_ptr, hctx_idx, io_queue);
        drop(lock);
        r
    }
    #[inline]
    fn _exit_hctx(&self, hctx_ptr: SafePtr, hctx_idx: usize, io_queue: bool) -> LinuxResult<()> {
        self.domain
            .read_directly(|domain| domain.exit_hctx(hctx_ptr, hctx_idx, io_queue))
    }
    #[inline]
    fn _exit_hctx_no_lock(
        &self,
        hctx_ptr: SafePtr,
        hctx_idx: usize,
        io_queue: bool,
    ) -> LinuxResult<()> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._exit_hctx(hctx_ptr, hctx_idx, io_queue);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }
    #[inline]
    fn _exit_hctx_with_lock(
        &self,
        hctx_ptr: SafePtr,
        hctx_idx: usize,
        io_queue: bool,
    ) -> LinuxResult<()> {
        let lock = self.lock.lock();
        let r = self._exit_hctx(hctx_ptr, hctx_idx, io_queue);
        drop(lock);
        r
    }
    #[inline]
    fn _queue_rq(
        &self,
        hctx_ptr: SafePtr,
        bd_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        self.domain.read_directly(|domain| {
            domain.queue_rq(hctx_ptr, bd_ptr, hctx_driver_data_ptr, io_queue)
        })
    }
    #[inline]
    fn _queue_rq_no_lock(
        &self,
        hctx_ptr: SafePtr,
        bd_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._queue_rq(hctx_ptr, bd_ptr, hctx_driver_data_ptr, io_queue);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }
    #[inline]
    fn _queue_rq_with_lock(
        &self,
        hctx_ptr: SafePtr,
        bd_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        let lock = self.lock.lock();
        let r = self._queue_rq(hctx_ptr, bd_ptr, hctx_driver_data_ptr, io_queue);
        drop(lock);
        r
    }

    #[inline]
    fn _commit_rqs(
        &self,
        hctx_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        self.domain
            .read_directly(|domain| domain.commit_rqs(hctx_ptr, hctx_driver_data_ptr, io_queue))
    }
    #[inline]
    fn _commit_rqs_no_lock(
        &self,
        hctx_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._commit_rqs(hctx_ptr, hctx_driver_data_ptr, io_queue);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }
    #[inline]
    fn _commit_rqs_with_lock(
        &self,
        hctx_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        let lock = self.lock.lock();
        let r = self._commit_rqs(hctx_ptr, hctx_driver_data_ptr, io_queue);
        drop(lock);
        r
    }
    #[inline]
    fn _complete_request(&self, rq_ptr: SafePtr, io_queue: bool) -> LinuxResult<()> {
        self.domain
            .read_directly(|domain| domain.complete_request(rq_ptr, io_queue))
    }
    #[inline]
    fn _complete_request_no_lock(&self, rq_ptr: SafePtr, io_queue: bool) -> LinuxResult<()> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._complete_request(rq_ptr, io_queue);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }
    #[inline]
    fn _complete_request_with_lock(&self, rq_ptr: SafePtr, io_queue: bool) -> LinuxResult<()> {
        let lock = self.lock.lock();
        let r = self._complete_request(rq_ptr, io_queue);
        drop(lock);
        r
    }
    #[inline]
    fn _map_queues(
        &self,
        tag_set_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        self.domain
            .read_directly(|domain| domain.map_queues(tag_set_ptr, driver_data_ptr, io_queue))
    }

    #[inline]
    fn _map_queues_with_lock(
        &self,
        tag_set_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        let lock = self.lock.lock();
        let r = self._map_queues(tag_set_ptr, driver_data_ptr, io_queue);
        drop(lock);
        r
    }
    #[inline]
    fn _map_queues_no_lock(
        &self,
        tag_set_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._map_queues(tag_set_ptr, driver_data_ptr, io_queue);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }
    #[inline]
    fn _poll_queues(
        &self,
        hctx_ptr: SafePtr,
        iob_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<i32> {
        self.domain
            .read_directly(|domain| domain.poll_queues(hctx_ptr, iob_ptr, io_queue))
    }
    #[inline]
    fn _poll_queues_with_lock(
        &self,
        hctx_ptr: SafePtr,
        iob_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<i32> {
        let lock = self.lock.lock();
        let r = self._poll_queues(hctx_ptr, iob_ptr, io_queue);
        drop(lock);
        r
    }
    #[inline]
    fn _poll_queues_no_lock(
        &self,
        hctx_ptr: SafePtr,
        iob_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<i32> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._poll_queues(hctx_ptr, iob_ptr, io_queue);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }
    #[inline]
    fn _handle_irq(&self, data: SafePtr) -> LinuxResult<u32> {
        self.domain.read_directly(|domain| domain.handle_irq(data))
    }
    #[inline]
    fn _handle_irq_with_lock(&self, data: SafePtr) -> LinuxResult<u32> {
        let lock = self.lock.lock();
        let r = self._handle_irq(data);
        drop(lock);
        r
    }
    #[inline]
    fn _handle_irq_no_lock(&self, data: SafePtr) -> LinuxResult<u32> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._handle_irq(data);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }
    #[inline]
    fn _probe(&self, pdev: SafePtr, pci_device_id: SafePtr) -> LinuxResult<i32> {
        self.domain
            .read_directly(|domain| domain.probe(pdev, pci_device_id))
    }
    #[inline]
    fn _probe_no_lock(&self, pdev: SafePtr, pci_device_id: SafePtr) -> LinuxResult<i32> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._probe(pdev, pci_device_id);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }
    #[inline]
    fn _probe_with_lock(&self, pdev: SafePtr, pci_device_id: SafePtr) -> LinuxResult<i32> {
        let lock = self.lock.lock();
        let r = self._probe(pdev, pci_device_id);
        drop(lock);
        r
    }
    #[inline]
    fn _remove(&self, pdev: SafePtr) -> LinuxResult<()> {
        self.domain.read_directly(|domain| domain.remove(pdev))
    }
    #[inline]
    fn _remove_no_lock(&self, pdev: SafePtr) -> LinuxResult<()> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._remove(pdev);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }
    #[inline]
    fn _remove_with_lock(&self, pdev: SafePtr) -> LinuxResult<()> {
        let lock = self.lock.lock();
        let r = self._remove(pdev);
        drop(lock);
        r
    }
}

impl IrqHandlerOp for NvmeDeviceDomainProxy {
    fn handle_irq(&self, data: SafePtr) -> LinuxResult<u32> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._handle_irq_with_lock(data)
        } else {
            self._handle_irq_no_lock(data)
        }
    }
}

impl PCIDeviceOp for NvmeDeviceDomainProxy {
    fn probe(&self, pdev: SafePtr, pci_device_id: SafePtr) -> LinuxResult<i32> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._probe_with_lock(pdev, pci_device_id)
        } else {
            self._probe_no_lock(pdev, pci_device_id)
        }
    }

    fn remove(&self, pdev: SafePtr) -> LinuxResult<()> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._remove_with_lock(pdev)
        } else {
            self._remove_no_lock(pdev)
        }
    }
}

impl BlkMqOp for NvmeDeviceDomainProxy {
    fn init_request(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._init_request_with_lock(tag_set_ptr, rq_ptr, driver_data_ptr, io_queue)
        } else {
            self._init_request_no_lock(tag_set_ptr, rq_ptr, driver_data_ptr, io_queue)
        }
    }
    fn exit_request(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._exit_request_with_lock(tag_set_ptr, rq_ptr, io_queue)
        } else {
            self._exit_request_no_lock(tag_set_ptr, rq_ptr, io_queue)
        }
    }
    fn init_hctx(
        &self,
        hctx_ptr: SafePtr,
        tag_set_data_ptr: SafePtr,
        hctx_idx: usize,
        io_queue: bool,
    ) -> LinuxResult<()> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._init_hctx_with_lock(hctx_ptr, tag_set_data_ptr, hctx_idx, io_queue)
        } else {
            self._init_hctx_no_lock(hctx_ptr, tag_set_data_ptr, hctx_idx, io_queue)
        }
    }

    fn exit_hctx(&self, hctx_ptr: SafePtr, hctx_idx: usize, io_queue: bool) -> LinuxResult<()> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._exit_hctx_with_lock(hctx_ptr, hctx_idx, io_queue)
        } else {
            self._exit_hctx_no_lock(hctx_ptr, hctx_idx, io_queue)
        }
    }
    fn queue_rq(
        &self,
        hctx_ptr: SafePtr,
        bd_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._queue_rq_with_lock(hctx_ptr, bd_ptr, hctx_driver_data_ptr, io_queue)
        } else {
            self._queue_rq_no_lock(hctx_ptr, bd_ptr, hctx_driver_data_ptr, io_queue)
        }
    }
    fn commit_rqs(
        &self,
        hctx_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._commit_rqs_with_lock(hctx_ptr, hctx_driver_data_ptr, io_queue)
        } else {
            self._commit_rqs_no_lock(hctx_ptr, hctx_driver_data_ptr, io_queue)
        }
    }
    fn complete_request(&self, rq_ptr: SafePtr, io_queue: bool) -> LinuxResult<()> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._complete_request_with_lock(rq_ptr, io_queue)
        } else {
            self._complete_request_no_lock(rq_ptr, io_queue)
        }
    }

    fn map_queues(
        &self,
        tag_set_ptr: SafePtr,
        driver_data_ptr: SafePtr,
        io_queue: bool,
    ) -> LinuxResult<()> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._map_queues_with_lock(tag_set_ptr, driver_data_ptr, io_queue)
        } else {
            self._map_queues_no_lock(tag_set_ptr, driver_data_ptr, io_queue)
        }
    }

    fn poll_queues(&self, hctx_ptr: SafePtr, iob_ptr: SafePtr, io_queue: bool) -> LinuxResult<i32> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._poll_queues_with_lock(hctx_ptr, iob_ptr, io_queue)
        } else {
            self._poll_queues_no_lock(hctx_ptr, iob_ptr, io_queue)
        }
    }
}

impl Basic for NvmeDeviceDomainProxy {
    fn domain_id(&self) -> u64 {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._domain_id_with_lock()
        } else {
            self._domain_id_no_lock()
        }
    }
}

impl NvmeBlockDeviceDomain for NvmeDeviceDomainProxy {
    fn init(&self, args: &NvmeBlockArgs) -> LinuxResult<()> {
        self.domain.read_directly(|domain| domain.init(args))
    }
    fn exit(&self) -> LinuxResult<()> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._exit_with_lock()
        } else {
            self._exit_no_lock()
        }
    }
}

impl NvmeDeviceDomainProxy {
    pub fn replace(
        &self,
        new_domain: Box<dyn NvmeBlockDeviceDomain>,
        domain_loader: DomainLoader,
    ) -> LinuxResult<()> {
        let mut loader_guard = self.domain_loader.lock();
        // The writer lock before enable the lock path
        let w_lock = self.lock.lock();
        let old_id = self.domain_id();
        // enable lock path
        self.flag.store(true, core::sync::atomic::Ordering::Relaxed);

        // wait all readers to finish
        while self.counter.sum() != 0 {
            println!("Wait for all reader to finish");
            // yield_now();
        }
        let resource = self.resource.get().unwrap();
        let args = resource.as_ref().downcast_ref::<NvmeBlockArgs>().unwrap();

        let new_domain_id = new_domain.domain_id();
        new_domain.init(args).unwrap();

        // stage4: swap the domain and change to normal state
        let old_domain = self.domain.update_directly(new_domain);

        // disable lock path
        self.flag
            .store(false, core::sync::atomic::Ordering::Relaxed);
        // stage5: recycle all resources
        let real_domain = Box::into_inner(old_domain);
        // forget the old domain, it will be dropped by the `free_domain_resource`
        forget(real_domain);

        // We should not free the shared data here, because the shared data will be used
        // in new domain.
        free_domain_resource(old_id, FreeShared::NotFree(new_domain_id));
        *loader_guard = domain_loader;
        drop(w_lock);
        drop(loader_guard);
        Ok(())
    }
}

#[derive(Debug)]
pub struct NvmeDeviceDomainEmptyImpl;

impl NvmeDeviceDomainEmptyImpl {
    pub fn new() -> Self {
        NvmeDeviceDomainEmptyImpl
    }
}

impl Basic for NvmeDeviceDomainEmptyImpl {
    fn domain_id(&self) -> u64 {
        u64::MAX
    }
}

impl IrqHandlerOp for NvmeDeviceDomainEmptyImpl {
    fn handle_irq(&self, _data: SafePtr) -> LinuxResult<u32> {
        Err(LinuxError::ENOSYS)
    }
}

impl PCIDeviceOp for NvmeDeviceDomainEmptyImpl {
    fn probe(&self, _pdev: SafePtr, _pci_device_id: SafePtr) -> LinuxResult<i32> {
        Err(LinuxError::ENOSYS)
    }

    fn remove(&self, _pdev: SafePtr) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }
}

impl BlkMqOp for NvmeDeviceDomainEmptyImpl {
    fn init_request(
        &self,
        _tag_set_ptr: SafePtr,
        _rq_ptr: SafePtr,
        _driver_data_ptr: SafePtr,
        _io_queue: bool,
    ) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn exit_request(
        &self,
        _tag_set_ptr: SafePtr,
        _rq_ptr: SafePtr,
        _io_queue: bool,
    ) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn init_hctx(
        &self,
        _hctx_ptr: SafePtr,
        _tag_set_data_ptr: SafePtr,
        _hctx_idx: usize,
        _io_queue: bool,
    ) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn exit_hctx(&self, _hctx_ptr: SafePtr, _hctx_idx: usize, _io_queue: bool) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn queue_rq(
        &self,
        _hctx_ptr: SafePtr,
        _bd_ptr: SafePtr,
        _hctx_driver_data_ptr: SafePtr,
        _io_queue: bool,
    ) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn commit_rqs(
        &self,
        _hctx_ptr: SafePtr,
        _hctx_driver_data_ptr: SafePtr,
        _io_queue: bool,
    ) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn complete_request(&self, _rq_ptr: SafePtr, _io_queue: bool) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn map_queues(
        &self,
        _tag_set_ptr: SafePtr,
        _driver_data_ptr: SafePtr,
        _io_queue: bool,
    ) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn poll_queues(
        &self,
        _hctx_ptr: SafePtr,
        _iob_ptr: SafePtr,
        _io_queue: bool,
    ) -> LinuxResult<i32> {
        Err(LinuxError::ENOSYS)
    }
}

impl NvmeBlockDeviceDomain for NvmeDeviceDomainEmptyImpl {
    fn init(&self, _args: &NvmeBlockArgs) -> LinuxResult<()> {
        Ok(())
    }
    fn exit(&self) -> LinuxResult<()> {
        Ok(())
    }
}
