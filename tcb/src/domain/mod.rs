use alloc::{boxed::Box, sync::Arc};

use corelib::LinuxResult;
use interface::{
    logger::{Level, LogDomain},
    Basic, DomainTypeRaw,
};
use kbind::env;
use rref::RRefVec;

use crate::{
    create_domain, domain_helper,
    domain_helper::{
        alloc_domain_id, free_domain_resource, FreeShared, DOMAIN_DATA_ALLOCATOR,
        SHARED_HEAP_ALLOCATOR,
    },
    domain_loader::{
        creator::{register_domain_elf, DomainCreateImpl},
        loader::{DomainCall, DomainLoader},
    },
    domain_proxy::{logger::LogDomainProxy, ProxyBuilder},
};

pub fn init_domain_system() -> LinuxResult<()> {
    init_basic_domains();
    init_kernel_domain();
    domain_helper::init_domain_create(Box::new(DomainCreateImpl));
    load_domains()?;
    Ok(())
}

static GNULL: &[u8] = include_bytes!("../../../build/disk/gnull");

/// Register the basic domains
fn init_basic_domains() {
    register_domain_elf("null", GNULL.to_vec(), DomainTypeRaw::EmptyDeviceDomain);
}

/// set the kernel to the specific domain
fn init_kernel_domain() {
    rref::init(SHARED_HEAP_ALLOCATOR, alloc_domain_id());
    storage::init_data_allocator(DOMAIN_DATA_ALLOCATOR);
}

pub fn load_domains() -> LinuxResult<()> {
    pr_info!("module_alloc func ptr: {:x?}", env::MODULE_ALLOC_ADDR);
    pr_info!("module_dealloc func ptr: {:x?}", env::MODULE_MEMFREE_ADDR);
    let null_domain = GNULL;
    pr_info!("The null domain size: {} bytes", null_domain.len());
    let mut loader = DomainLoader::new(Arc::new(null_domain.to_vec()), "gnull");
    loader.load().unwrap();
    loader.call_raw();
    pr_info!("The null domain is loaded");
    Ok(())
}
