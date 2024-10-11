use alloc::boxed::Box;
use core::{any::Any, mem::forget, pin::Pin};

use corelib::{LinuxErrno, LinuxResult};
use interface::{logger::LogDomain, Basic};
use kernel::{
    init::InPlaceInit,
    sync::{Mutex, RcuData},
};
use rref::RRefVec;

use crate::{
    domain_helper::{free_domain_resource, FreeShared},
    domain_loader::loader::DomainLoader,
    domain_proxy::ProxyBuilder,
};

#[derive(Debug)]
pub struct LogDomainProxy {
    domain: RcuData<Box<dyn LogDomain>>,
    domain_loader: Pin<Box<Mutex<DomainLoader>>>,
}

impl LogDomainProxy {
    pub fn new(domain: Box<dyn LogDomain>, domain_loader: DomainLoader) -> Self {
        LogDomainProxy {
            domain: RcuData::new(domain),
            domain_loader: Box::pin_init(new_mutex!(domain_loader)).unwrap(),
        }
    }
    pub fn domain_loader(&self) -> DomainLoader {
        self.domain_loader.lock().clone()
    }
}

impl Basic for LogDomainProxy {
    fn domain_id(&self) -> u64 {
        self.domain.read(|domain| domain.domain_id())
    }
}

impl LogDomain for LogDomainProxy {
    fn init(&self) -> LinuxResult<()> {
        self.domain.read(|domain| domain.init())
    }

    fn log(&self, level: interface::logger::Level, msg: &RRefVec<u8>) -> LinuxResult<()> {
        self.domain.read(|domain| domain.log(level, msg))
    }

    fn set_max_level(&self, level: interface::logger::LevelFilter) -> LinuxResult<()> {
        self.domain.read(|domain| domain.set_max_level(level))
    }
}

impl LogDomainProxy {
    pub fn replace(
        &self,
        new_domain: Box<dyn LogDomain>,
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
        Err(LinuxErrno::ENOSYS)
    }

    fn set_max_level(&self, _level: interface::logger::LevelFilter) -> LinuxResult<()> {
        Err(LinuxErrno::ENOSYS)
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
    fn build_empty_no_proxy() -> Self::T {
        Box::new(LogDomainEmptyImpl::new())
    }
    fn init_by_box(&self, argv: Box<dyn Any + Send + Sync>) -> LinuxResult<()> {
        let _ = argv;
        self.init()
    }
}
