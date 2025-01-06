use alloc::{boxed::Box, vec::Vec};
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
use kernel::str::CStr;
pub use nvme::NvmeDomainShim;

pub struct KObj {
    entropy_source: Sysctl<EntropySource>,
    one_device: Vec<Sysctl<OneDevice>>,
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
    // let one_device = OneDevice::new(empty_device);
    // let one_device = Sysctl::register(
    //     c_str!("rust/domain"),
    //     c_str!("one"),
    //     one_device,
    //     Mode::from_int(0o666),
    // )?;

    const ONE_NAME: &[&'static CStr] = &[
        c_str!("one0"),
        c_str!("one1"),
        c_str!("one2"),
        c_str!("one3"),
        c_str!("one4"),
        c_str!("one5"),
        c_str!("one6"),
        c_str!("one7"),
        c_str!("one8"),
        c_str!("one9"),
        c_str!("one10"),
        c_str!("one11"),
        c_str!("one12"),
        c_str!("one13"),
        c_str!("one14"),
        c_str!("one15"),
        c_str!("one16"),
        c_str!("one17"),
        c_str!("one18"),
        c_str!("one19"),
        c_str!("one20"),
        c_str!("one21"),
        c_str!("one22"),
        c_str!("one23"),
        c_str!("one24"),
        c_str!("one25"),
        c_str!("one26"),
        c_str!("one27"),
        c_str!("one28"),
        c_str!("one29"),
        c_str!("one30"),
        c_str!("one31"),
        c_str!("one32"),
    ];

    let mut one_device_list = Vec::new();
    for i in 0..32 {
        let one_device = OneDevice::new(empty_device.clone());
        let one_device = Sysctl::register(
            c_str!("rust/domain"),
            ONE_NAME[i],
            one_device,
            Mode::from_int(0o666),
        )?;
        one_device_list.push(one_device);
    }

    println!("One device registered");
    Ok(KObj {
        entropy_source: entropy,
        one_device: one_device_list,
    })
}

pub trait KernelShim: Send + Sync {
    fn any(self: Box<Self>) -> Box<dyn Any>;
    fn domain_type(&self) -> DomainTypeRaw;
}
