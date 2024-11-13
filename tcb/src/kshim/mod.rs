use alloc::boxed::Box;
use core::any::Any;

use interface::{DomainType, DomainTypeRaw};
use kernel::{
    error::{linux_err, KernelResult},
    sysctl::Sysctl,
    types::Mode,
};

use crate::{
    domain_helper::query_domain,
    kshim::{entropy::EntropySource, one::OneDevice},
};

mod block_device;
mod entropy;
mod nvme;
mod one;

pub use block_device::BlockDeviceShim;
pub use nvme::NvmeDomainShim;

pub struct KObj {
    entropy_source: Sysctl<EntropySource>,
    one_device: Sysctl<OneDevice>,
}

pub fn init_kernel_shim() -> KernelResult<KObj> {
    let domain = query_domain("logger").unwrap();
    let log_domain = match domain {
        DomainType::LogDomain(log_domain) => log_domain,
        _ => {
            pr_err!("Failed to get logger domain");
            return Err(linux_err::EINVAL);
        }
    };
    let entropy = EntropySource::new(log_domain);
    let entropy = Sysctl::register(
        c_str!("rust/domain"),
        c_str!("entropy"),
        entropy,
        Mode::from_int(0o666),
    )?;
    println!("Entropy source registered");

    let empty_device = query_domain("empty_device").unwrap();
    let empty_device = match empty_device {
        DomainType::EmptyDeviceDomain(empty_device) => empty_device,
        _ => {
            pr_err!("Failed to get empty device domain");
            return Err(linux_err::EINVAL);
        }
    };
    let one_device = OneDevice::new(empty_device);
    let one_device = Sysctl::register(
        c_str!("rust/domain"),
        c_str!("one"),
        one_device,
        Mode::from_int(0o666),
    )?;
    println!("One device registered");
    Ok(KObj {
        entropy_source: entropy,
        one_device,
    })
}

pub trait KernelShim: Send + Sync {
    fn any(self: Box<Self>) -> Box<dyn Any>;
    fn domain_type(&self) -> DomainTypeRaw;
}
