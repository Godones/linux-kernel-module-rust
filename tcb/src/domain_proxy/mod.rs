use alloc::boxed::Box;
use core::any::Any;

use corelib::LinuxResult;
use interface::{logger::LogDomain, Basic};
use ksync::Mutex;
use rref::RRefVec;

use crate::domain_loader::loader::DomainLoader;

mod scheduler;

pub trait ProxyBuilder {
    type T;
    fn build(domain: Self::T, domain_loader: DomainLoader) -> Self;
    fn build_empty(domain_loader: DomainLoader) -> Self;
    fn init_by_box(&self, argv: Box<dyn Any + Send + Sync>) -> LinuxResult<()>;
}
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
