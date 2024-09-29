use alloc::boxed::Box;

use corelib::LinuxResult;
use kbind::env;

use crate::{
    domain_helper,
    domain_helper::{alloc_domain_id, DOMAIN_DATA_ALLOCATOR, SHARED_HEAP_ALLOCATOR},
    domain_loader::creator::DomainCreateImpl,
};

pub fn init_domain_system() -> LinuxResult<()> {
    init_kernel_domain();
    domain_helper::init_domain_create(Box::new(DomainCreateImpl));
    pr_info!("module_alloc func ptr: {:x?}", env::MODULE_ALLOC_ADDR);
    pr_info!("module_dealloc func ptr: {:x?}", env::MODULE_MEMFREE_ADDR);
    Ok(())
}

/// set the kernel to the specific domain
fn init_kernel_domain() {
    rref::init(SHARED_HEAP_ALLOCATOR, alloc_domain_id());
    storage::init_data_allocator(DOMAIN_DATA_ALLOCATOR);
}
