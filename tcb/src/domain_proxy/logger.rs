use alloc::boxed::Box;
use core::any::Any;

use corelib::LinuxResult;
use interface::{logger::LogDomain, Basic};
use ksync::Mutex;
use rref::RRefVec;

use crate::{domain_loader::loader::DomainLoader, domain_proxy::ProxyBuilder};

#[derive(Debug)]
pub struct LogDomainProxy {
    domain: Box<dyn LogDomain>,
    domain_loader: Mutex<DomainLoader>,
}

impl LogDomainProxy {
    pub fn replace(
        &self,
        domain: Box<dyn LogDomain>,
        domain_loader: DomainLoader,
    ) -> LinuxResult<()> {
        // *self.domain_loader.lock() = domain_loader;
        // self.domain = domain;
        unimplemented!("replace LogDomainProxy")
    }
    pub fn new(domain: Box<dyn LogDomain>, domain_loader: DomainLoader) -> Self {
        LogDomainProxy {
            domain,
            domain_loader: Mutex::new(domain_loader),
        }
    }
}

impl Basic for LogDomainProxy {
    fn domain_id(&self) -> u64 {
        self.domain.domain_id()
    }
}

impl LogDomain for LogDomainProxy {
    fn init(&self) -> LinuxResult<()> {
        self.domain.init()
    }

    fn log(&self, level: interface::logger::Level, msg: &RRefVec<u8>) -> LinuxResult<()> {
        self.domain.log(level, msg)
    }

    fn set_max_level(&self, level: interface::logger::LevelFilter) -> LinuxResult<()> {
        self.domain.set_max_level(level)
    }
}
#[derive(Debug)]
pub struct LogDomainEmptyImpl;
impl LogDomainEmptyImpl {
    pub fn new() -> Self {
        LogDomainEmptyImpl
    }
}
impl Basic for LogDomainEmptyImpl {
    fn domain_id(&self) -> u64 {
        u64::MAX
    }
}

impl LogDomain for LogDomainEmptyImpl {
    fn init(&self) -> LinuxResult<()> {
        Ok(())
    }

    fn log(&self, _level: interface::logger::Level, _msg: &RRefVec<u8>) -> LinuxResult<()> {
        Ok(())
    }

    fn set_max_level(&self, _level: interface::logger::LevelFilter) -> LinuxResult<()> {
        Ok(())
    }
}

impl ProxyBuilder for LogDomainProxy {
    type T = Box<dyn LogDomain>;

    fn build(domain: Self::T, domain_loader: DomainLoader) -> Self {
        Self::new(domain, domain_loader)
    }

    fn build_empty(domain_loader: DomainLoader) -> Self {
        let domain = Box::new(LogDomainEmptyImpl::new());
        Self::new(domain, domain_loader)
    }

    fn init_by_box(&self, argv: Box<dyn Any + Send + Sync>) -> LinuxResult<()> {
        let _ = argv;
        self.init()
    }
}
