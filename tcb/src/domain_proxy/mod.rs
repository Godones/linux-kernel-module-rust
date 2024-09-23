use alloc::boxed::Box;
use core::any::Any;

use corelib::LinuxResult;

use crate::domain_loader::loader::DomainLoader;

pub mod empty_device;
pub mod logger;
mod scheduler;

pub trait ProxyBuilder {
    type T;
    fn build(domain: Self::T, domain_loader: DomainLoader) -> Self;
    fn build_empty(domain_loader: DomainLoader) -> Self;
    fn init_by_box(&self, argv: Box<dyn Any + Send + Sync>) -> LinuxResult<()>;
}
