use alloc::boxed::Box;
use core::{any::Any, mem::forget};

use corelib::{LinuxError, LinuxResult};
use interface::{empty_device::EmptyDeviceDomain, Basic};
use kbind::sync::{RcuData, Spinlock};
use rref::RRefVec;

use crate::{
    domain_helper::{free_domain_resource, FreeShared},
    domain_loader::loader::DomainLoader,
    domain_proxy::ProxyBuilder,
};

#[derive(Debug)]
pub struct EmptyDeviceDomainProxy {
    domain: RcuData<Box<dyn EmptyDeviceDomain>>,
    domain_loader: Spinlock<DomainLoader>,
}

impl EmptyDeviceDomainProxy {
    pub fn new(domain: Box<dyn EmptyDeviceDomain>, domain_loader: DomainLoader) -> Self {
        EmptyDeviceDomainProxy {
            domain: RcuData::new(domain),
            domain_loader: Spinlock::new(domain_loader),
        }
    }
}

impl ProxyBuilder for EmptyDeviceDomainProxy {
    type T = Box<dyn EmptyDeviceDomain>;

    fn build(domain: Self::T, domain_loader: DomainLoader) -> Self {
        Self::new(domain, domain_loader)
    }

    fn build_empty(domain_loader: DomainLoader) -> Self {
        Self::new(Box::new(EmptyDeviceDomainEmptyImpl::new()), domain_loader)
    }

    fn init_by_box(&self, _argv: Box<dyn Any + Send + Sync>) -> LinuxResult<()> {
        self.init()
    }
}

impl Basic for EmptyDeviceDomainProxy {
    fn domain_id(&self) -> u64 {
        self.domain.read(|domain| domain.domain_id())
    }
}

impl EmptyDeviceDomain for EmptyDeviceDomainProxy {
    fn init(&self) -> LinuxResult<()> {
        self.domain.read(|domain| domain.init())
    }

    fn read(&self, data: RRefVec<u8>) -> LinuxResult<RRefVec<u8>> {
        self.domain.read(|domain| domain.read(data))
    }

    fn write(&self, data: &RRefVec<u8>) -> LinuxResult<usize> {
        self.domain.read(|domain| domain.write(data))
    }
}

impl EmptyDeviceDomainProxy {
    pub fn replace(
        &self,
        new_domain: Box<dyn EmptyDeviceDomain>,
        domain_loader: DomainLoader,
    ) -> LinuxResult<()> {
        let mut loader_guard = self.domain_loader.lock();
        let old_id = self.domain_id();
        // init new domain
        new_domain.init().unwrap();
        // swap domain
        let old_domain = self.domain.update(new_domain);
        // free old domain
        let real_domain = Box::into_inner(old_domain);
        forget(real_domain);
        free_domain_resource(old_id, FreeShared::Free);
        *loader_guard = domain_loader;
        Ok(())
    }
}

#[derive(Debug)]
pub struct EmptyDeviceDomainEmptyImpl;

impl EmptyDeviceDomainEmptyImpl {
    pub fn new() -> Self {
        EmptyDeviceDomainEmptyImpl
    }
}

impl Basic for EmptyDeviceDomainEmptyImpl {
    fn domain_id(&self) -> u64 {
        u64::MAX
    }
}

impl EmptyDeviceDomain for EmptyDeviceDomainEmptyImpl {
    fn init(&self) -> LinuxResult<()> {
        Ok(())
    }

    fn read(&self, _data: RRefVec<u8>) -> LinuxResult<RRefVec<u8>> {
        Err(LinuxError::ENOSYS)
    }

    fn write(&self, _data: &RRefVec<u8>) -> LinuxResult<usize> {
        Err(LinuxError::ENOSYS)
    }
}
