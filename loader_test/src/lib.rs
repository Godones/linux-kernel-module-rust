#![no_std]
extern crate alloc;

#[macro_use]
extern crate log;

use linux_kernel_module::{code, logger, println};

mod domain_loader;
mod mm;

struct DomainLoaderModule;

impl linux_kernel_module::KernelModule for DomainLoaderModule {
    fn init() -> linux_kernel_module::KernelResult<Self> {
        println!("load domain test module");
        logger::init_logger();
        let err = code::EINVAL;
        println!("error: {:?}", err);
        domain_loader::load_domain();
        Ok(DomainLoaderModule)
    }
}

impl Drop for DomainLoaderModule {
    fn drop(&mut self) {
        println!("unload domain test module");
    }
}

linux_kernel_module::kernel_module!(
    DomainLoaderModule,
    author: b"godones",
    description: b"DomainLoaderModule",
    license: b"GPL"
);
