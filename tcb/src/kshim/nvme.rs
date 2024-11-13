use alloc::{boxed::Box, sync::Arc};
use core::any::Any;

use interface::{nvme::NvmeBlockDeviceDomain, DomainTypeRaw};

use crate::kshim::KernelShim;

pub struct NvmeDomainShim {
    nvme: Arc<dyn NvmeBlockDeviceDomain>,
}

impl NvmeDomainShim {
    pub fn new(nvme: Arc<dyn NvmeBlockDeviceDomain>) -> Self {
        NvmeDomainShim { nvme }
    }
}

impl KernelShim for NvmeDomainShim {
    fn any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
    fn domain_type(&self) -> DomainTypeRaw {
        DomainTypeRaw::NvmeBlockDeviceDomain
    }
}
