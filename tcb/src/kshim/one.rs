use alloc::sync::Arc;

use interface::empty_device::EmptyDeviceDomain;
use kbind::{
    kernel_ptr::KernelSlicePtrWriter, linux_err::EINVAL, sync::CpuId, sysctl::SysctlStorage,
    KernelResult,
};
use rref::RRefVec;

pub struct OneDevice {
    domain: Arc<dyn EmptyDeviceDomain>,
}

impl OneDevice {
    pub fn new(domain: Arc<dyn EmptyDeviceDomain>) -> Self {
        Self { domain }
    }
}

impl SysctlStorage for OneDevice {
    fn store_value(&self, data: &[u8]) -> (usize, KernelResult<()>) {
        let str = core::str::from_utf8(data).unwrap();
        CpuId::read(|id| {
            println!("[core: {}] OneDevice::store_value: {}", id, str);
        });
        let rvec = RRefVec::from_slice(data);
        let r = self.domain.write(&rvec);
        if let Ok(r) = r {
            (r, Ok(()))
        } else {
            (0, Err(EINVAL))
        }
    }
    fn read_value(&self, data: &mut KernelSlicePtrWriter) -> (usize, KernelResult<()>) {
        let rvec = RRefVec::new_uninit(data.len());
        let r = self.domain.read(rvec);
        if let Ok(r) = r {
            (r.len(), data.write(r.as_slice()))
        } else {
            (0, Err(EINVAL))
        }
    }
}
