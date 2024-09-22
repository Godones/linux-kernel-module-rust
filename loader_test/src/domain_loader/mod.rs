use alloc::sync::Arc;

use linux_kernel_module::{env, pr_info};

use crate::domain_loader::loader::DomainLoader;

mod loader;

static GNULL: &[u8] = include_bytes!("../../../build/disk/gnull");

pub fn load_domain() {
    pr_info!("module_alloc: {:x?}", env::MODULE_ALLOC_ADDR);
    pr_info!("module_dealloc: {:x?}", env::MODULE_MEMFREE_ADDR);

    let null_domain = GNULL;
    pr_info!("The null domain size: {} bytes", null_domain.len());
    let mut loader = DomainLoader::new(Arc::new(null_domain.to_vec()), "gnull");
    loader.load().unwrap();
    loader.call();

    pr_info!("load_domain done");
}
