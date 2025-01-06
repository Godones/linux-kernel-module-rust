#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;

use alloc::boxed::Box;
use core::fmt::Debug;

use basic::{console::println, LinuxResult};
use interface::{empty_device::EmptyDeviceDomain, Basic};
use rref::RRefVec;
use spin::Mutex;

pub struct NullDeviceDomainImpl {
    fake_mem: Mutex<[u8; 4096]>,
}

impl NullDeviceDomainImpl {
    pub fn new() -> Self {
        Self {
            fake_mem: Mutex::new([0; 4096]),
        }
    }

    pub fn do_read(&self, mut data: RRefVec<u8>) -> LinuxResult<RRefVec<u8>> {
        let fake_mem = self.fake_mem.lock();
        let copy_len = core::cmp::min(data.len(), fake_mem.len());
        data.as_mut_slice()[..copy_len].copy_from_slice(&fake_mem[..copy_len]);
        Ok(data)
    }

    pub fn do_write(&self, data: &RRefVec<u8>) -> LinuxResult<usize> {
        let mut fake_mem = self.fake_mem.lock();
        let copy_len = core::cmp::min(data.len(), fake_mem.len());
        fake_mem[..copy_len].copy_from_slice(&data.as_slice()[..copy_len]);
        Ok(copy_len)
    }
}

impl Debug for NullDeviceDomainImpl {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "NullDeviceDomainImpl")
    }
}

impl Basic for NullDeviceDomainImpl {
    fn domain_id(&self) -> u64 {
        rref::domain_id()
    }
}

impl EmptyDeviceDomain for NullDeviceDomainImpl {
    fn init(&self) -> LinuxResult<()> {
        // println!("NullDeviceDomainImpl init");
        Ok(())
    }

    fn read(&self, data: RRefVec<u8>) -> LinuxResult<RRefVec<u8>> {
        self.do_read(data)
    }
    fn write(&self, data: &RRefVec<u8>) -> LinuxResult<usize> {
        // println!("NullDeviceDomainImpl write");
        self.do_write(data)
    }
}
#[derive(Debug)]
pub struct UnwindWrap(NullDeviceDomainImpl);

impl UnwindWrap {
    pub fn new(real: NullDeviceDomainImpl) -> Self {
        Self(real)
    }
}
impl Basic for UnwindWrap {
    fn domain_id(&self) -> u64 {
        self.0.domain_id()
    }
}
impl EmptyDeviceDomain for UnwindWrap {
    fn init(&self) -> LinuxResult<()> {
        self.0.init()
    }
    fn read(&self, data: RRefVec<u8>) -> LinuxResult<RRefVec<u8>> {
        basic::catch_unwind(|| self.0.read(data))
    }
    fn write(&self, data: &RRefVec<u8>) -> LinuxResult<usize> {
        basic::catch_unwind(|| self.0.write(data))
    }
}

pub fn main() -> Box<dyn EmptyDeviceDomain> {
    Box::new(UnwindWrap::new(NullDeviceDomainImpl::new()))
}
