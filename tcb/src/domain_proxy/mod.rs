use alloc::boxed::Box;
use core::any::Any;

use corelib::LinuxResult;

use crate::domain_loader::loader::DomainLoader;

pub mod empty_device;
pub mod logger;

pub trait ProxyBuilder {
    type T;
    fn build(domain: Self::T, domain_loader: DomainLoader) -> Self;
    fn build_empty(domain_loader: DomainLoader) -> Self;
    fn build_empty_no_proxy() -> Self::T;
    fn init_by_box(&self, argv: Box<dyn Any + Send + Sync>) -> LinuxResult<()>;
}
