use alloc::boxed::Box;
use core::{any::Any, mem::forget, pin::Pin, sync::atomic::AtomicBool};

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

impl BlockDeviceDomain for BlockDeviceDomainProxy {
    fn init(&self, args: &BlockArgs) -> LinuxResult<()> {
        self.domain.read(|domain| domain.init(args))
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
    fn exit(&self) -> LinuxResult<()> {
        Ok(())
    }
}
