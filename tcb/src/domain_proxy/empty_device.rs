use alloc::boxed::Box;
use core::any::Any;

use corelib::{LinuxError, LinuxResult};
use interface::{empty_device::EmptyDeviceDomain, Basic};
use ksync::Mutex;
use rref::RRefVec;

use crate::{domain_loader::loader::DomainLoader, domain_proxy::ProxyBuilder};

#[derive(Debug)]
pub struct EmptyDeviceDomainProxy {
    domain: Box<dyn EmptyDeviceDomain>,
    domain_loader: Mutex<DomainLoader>,
}

impl EmptyDeviceDomainProxy {
    pub fn replace(
        &self,
        domain: Box<dyn EmptyDeviceDomain>,
        domain_loader: DomainLoader,
    ) -> LinuxResult<()> {
        // *self.domain_loader.lock() = domain_loader;
        // self.domain = domain;
        unimplemented!("replace EmptyDeviceDomainProxy")
    }
    pub fn new(domain: Box<dyn EmptyDeviceDomain>, domain_loader: DomainLoader) -> Self {
        EmptyDeviceDomainProxy {
            domain,
            domain_loader: Mutex::new(domain_loader),
        }
    }
}

impl Basic for EmptyDeviceDomainProxy {
    fn domain_id(&self) -> u64 {
        self.domain.domain_id()
    }
}

impl EmptyDeviceDomain for EmptyDeviceDomainProxy {
    fn init(&self) -> LinuxResult<()> {
        self.domain.init()
    }

    fn read(&self, data: RRefVec<u8>) -> LinuxResult<RRefVec<u8>> {
        self.domain.read(data)
    }

    fn write(&self, data: &RRefVec<u8>) -> LinuxResult<usize> {
        self.domain.write(data)
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

impl ProxyBuilder for EmptyDeviceDomainProxy {
    type T = EmptyDeviceDomainProxy;

    fn build(domain: Self::T, domain_loader: DomainLoader) -> Self {
        Self::new(domain.domain, domain_loader)
    }

    fn build_empty(domain_loader: DomainLoader) -> Self {
        Self::new(Box::new(EmptyDeviceDomainEmptyImpl::new()), domain_loader)
    }

    fn init_by_box(&self, _argv: Box<dyn Any + Send + Sync>) -> LinuxResult<()> {
        self.init()
    }
}
