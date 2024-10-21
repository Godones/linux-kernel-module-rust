use alloc::boxed::Box;

use corelib::LinuxResult;
use interface::{null_block::BlockDeviceDomain, DomainType, DomainTypeRaw};
use kernel::env;

use crate::{
    create_domain, domain_helper,
    domain_helper::{alloc_domain_id, DOMAIN_DATA_ALLOCATOR, SHARED_HEAP_ALLOCATOR},
    domain_loader::creator::DomainCreateImpl,
    domain_proxy::{empty_device::EmptyDeviceDomainProxy, logger::LogDomainProxy, ProxyBuilder},
    register_domain,
};

pub fn init_domain_system() -> LinuxResult<()> {
    init_kernel_domain();
    domain_helper::init_domain_create(Box::new(DomainCreateImpl));
    pr_info!("module_alloc func ptr: {:x?}", env::MODULE_ALLOC_ADDR);
    pr_info!("module_dealloc func ptr: {:x?}", env::MODULE_MEMFREE_ADDR);

    let (logger, domain_file_info) =
        create_domain!(LogDomainProxy, DomainTypeRaw::LogDomain, "logger")?;
    logger.init_by_box(Box::new(()))?;
    register_domain!(
        "logger",
        domain_file_info,
        DomainType::LogDomain(logger),
        true
    );
    println!("Register a empty logger domain");

    let (null_device, domain_file_info) = create_domain!(
        EmptyDeviceDomainProxy,
        DomainTypeRaw::EmptyDeviceDomain,
        "empty_device"
    )?;
    null_device.init_by_box(Box::new(()))?;
    register_domain!(
        "empty_device",
        domain_file_info,
        DomainType::EmptyDeviceDomain(null_device),
        true
    );
    println!("Register a empty device domain");

    Ok(())
}

/// set the kernel to the specific domain
fn init_kernel_domain() {
    rref::init(SHARED_HEAP_ALLOCATOR, alloc_domain_id());
    storage::init_data_allocator(DOMAIN_DATA_ALLOCATOR);
}
