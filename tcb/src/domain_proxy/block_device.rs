use alloc::boxed::Box;
use core::{any::Any, mem::forget, pin::Pin, sync::atomic::AtomicBool};

use basic::SafePtr;
use corelib::{LinuxError, LinuxResult};
use interface::{
    null_block::{BlockArgs, BlockDeviceDomain},
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
pub struct BlockDeviceDomainProxy {
    domain: SRcuData<Box<dyn BlockDeviceDomain>>,
    lock: Pin<Box<Mutex<()>>>,
    domain_loader: Pin<Box<Mutex<DomainLoader>>>,
    flag: AtomicBool,
    counter: LongLongPerCpu,
    resource: Once<Box<dyn Any + Send + Sync>>,
}

impl BlockDeviceDomainProxy {
    pub fn new(domain: Box<dyn BlockDeviceDomain>, domain_loader: DomainLoader) -> Self {
        BlockDeviceDomainProxy {
            domain: SRcuData::new(domain),
            lock: Box::pin_init(new_mutex!(())).unwrap(),
            domain_loader: Box::pin_init(new_mutex!(domain_loader)).unwrap(),
            flag: AtomicBool::new(false),
            counter: LongLongPerCpu::new(),
            resource: Once::new(),
        }
    }
}
impl ProxyBuilder for BlockDeviceDomainProxy {
    type T = Box<dyn BlockDeviceDomain>;

    fn build(domain: Self::T, domain_loader: DomainLoader) -> Self {
        Self::new(domain, domain_loader)
    }

    fn build_empty(domain_loader: DomainLoader) -> Self {
        Self::new(Box::new(BlockDeviceDomainEmptyImpl::new()), domain_loader)
    }
    fn build_empty_no_proxy() -> Self::T {
        Box::new(BlockDeviceDomainEmptyImpl::new())
    }

    fn init_by_box(&self, argv: Box<dyn Any + Send + Sync>) -> LinuxResult<()> {
        let args = argv.downcast_ref::<BlockArgs>().ok_or(LinuxError::EINVAL)?;
        self.init(args)?;
        self.resource.call_once(|| argv);
        Ok(())
    }
}

impl Basic for BlockDeviceDomainProxy {
    fn domain_id(&self) -> u64 {
        self.domain.read(|domain| domain.domain_id())
    }
}

impl BlockDeviceDomainProxy {}

impl BlockDeviceDomain for BlockDeviceDomainProxy {
    fn init(&self, args: &BlockArgs) -> LinuxResult<()> {
        self.domain.read(|domain| domain.init(args))
    }
    fn tag_set_with_queue_data(&self) -> LinuxResult<(SafePtr, SafePtr)> {
        self.domain.read(|domain| domain.tag_set_with_queue_data())
    }
    fn set_gen_disk(&self, gen_disk: SafePtr) -> LinuxResult<()> {
        self.domain.read(|domain| domain.set_gen_disk(gen_disk))
    }
    fn open(&self, mode: u32) -> LinuxResult<()> {
        self.domain.read(|domain| domain.open(mode))
    }
    fn release(&self) -> LinuxResult<()> {
        self.domain.read(|domain| domain.release())
    }
    fn init_request(
        &self,
        tag_set_ptr: SafePtr,
        rq_ptr: SafePtr,
        driver_data_ptr: SafePtr,
    ) -> LinuxResult<()> {
        self.domain
            .read(|domain| domain.init_request(tag_set_ptr, rq_ptr, driver_data_ptr))
    }
    fn exit_request(&self, tag_set_ptr: SafePtr, rq_ptr: SafePtr) -> LinuxResult<()> {
        self.domain
            .read(|domain| domain.exit_request(tag_set_ptr, rq_ptr))
    }
    fn init_hctx(
        &self,
        hctx_ptr: SafePtr,
        tag_set_data_ptr: SafePtr,
        hctx_idx: usize,
    ) -> LinuxResult<()> {
        self.domain
            .read(|domain| domain.init_hctx(hctx_ptr, tag_set_data_ptr, hctx_idx))
    }

    fn exit_hctx(&self, hctx_ptr: SafePtr, hctx_idx: usize) -> LinuxResult<()> {
        self.domain
            .read(|domain| domain.exit_hctx(hctx_ptr, hctx_idx))
    }
    fn queue_rq(
        &self,
        hctx_ptr: SafePtr,
        bd_ptr: SafePtr,
        hctx_driver_data_ptr: SafePtr,
    ) -> LinuxResult<()> {
        self.domain
            .read(|domain| domain.queue_rq(hctx_ptr, bd_ptr, hctx_driver_data_ptr))
    }
    fn commit_rqs(&self, hctx_ptr: SafePtr, hctx_driver_data_ptr: SafePtr) -> LinuxResult<()> {
        self.domain
            .read(|domain| domain.commit_rqs(hctx_ptr, hctx_driver_data_ptr))
    }
    fn complete_request(&self, rq_ptr: SafePtr) -> LinuxResult<()> {
        self.domain.read(|domain| domain.complete_request(rq_ptr))
    }
    fn exit(&self) -> LinuxResult<()> {
        self.domain.read(|domain| domain.exit())
    }
}

impl BlockDeviceDomainProxy {
    pub fn replace(
        &self,
        new_domain: Box<dyn BlockDeviceDomain>,
        domain_loader: DomainLoader,
    ) -> LinuxResult<()> {
        let mut loader_guard = self.domain_loader.lock();
        // The writer lock before enable the lock path
        let w_lock = self.lock.lock();
        // enable lock path
        self.flag.store(true, core::sync::atomic::Ordering::Relaxed);

        // wait all readers to finish
        while self.counter.sum() != 0 {
            println!("Wait for all reader to finish");
            // yield_now();
        }
        let old_id = self.domain_id();
        let resource = self.resource.get().unwrap();
        let args = resource.as_ref().downcast_ref::<BlockArgs>().unwrap();

        let new_domain_id = new_domain.domain_id();
        new_domain.init(args).unwrap();

        // stage4: swap the domain and change to normal state
        let old_domain = self.domain.update(new_domain);

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
pub struct BlockDeviceDomainEmptyImpl;

impl BlockDeviceDomainEmptyImpl {
    pub fn new() -> Self {
        BlockDeviceDomainEmptyImpl
    }
}

impl Basic for BlockDeviceDomainEmptyImpl {
    fn domain_id(&self) -> u64 {
        u64::MAX
    }
}

impl BlockDeviceDomain for BlockDeviceDomainEmptyImpl {
    fn init(&self, _args: &BlockArgs) -> LinuxResult<()> {
        Ok(())
    }
    fn tag_set_with_queue_data(&self) -> LinuxResult<(SafePtr, SafePtr)> {
        Err(LinuxError::ENOSYS)
    }
    fn set_gen_disk(&self, _gen_disk: SafePtr) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn open(&self, _mode: u32) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn release(&self) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn init_request(
        &self,
        _tag_set_ptr: SafePtr,
        _rq_ptr: SafePtr,
        _driver_data_ptr: SafePtr,
    ) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn exit_request(&self, _tag_set_ptr: SafePtr, _rq_ptr: SafePtr) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn init_hctx(
        &self,
        _hctx_ptr: SafePtr,
        _tag_set_data_ptr: SafePtr,
        _hctx_idx: usize,
    ) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn exit_hctx(&self, _hctx_driver_data_ptr: SafePtr, _hctx_idx: usize) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn queue_rq(
        &self,
        _hctx_ptr: SafePtr,
        _bd_ptr: SafePtr,
        _hctx_driver_data_ptr: SafePtr,
    ) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn commit_rqs(&self, _hctx_ptr: SafePtr, _hctx_driver_data_ptr: SafePtr) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }
    fn complete_request(&self, _rq_ptr: SafePtr) -> LinuxResult<()> {
        Err(LinuxError::ENOSYS)
    }

    fn exit(&self) -> LinuxResult<()> {
        Ok(())
    }
}
