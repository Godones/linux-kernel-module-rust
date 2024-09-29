#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;

use alloc::boxed::Box;
use core::fmt::Debug;

use basic::{LinuxResult};
use interface::{ Basic};
use interface::empty_device::EmptyDeviceDomain;
use rref::RRefVec;

#[derive(Debug)]
pub struct NullDeviceDomainImpl;

impl Basic for NullDeviceDomainImpl {
    fn domain_id(&self) -> u64 {
        rref::domain_id()
    }
}

impl EmptyDeviceDomain for NullDeviceDomainImpl {
    fn init(&self) -> LinuxResult<()> {
        Ok(())
    }

    fn read(&self, mut data: RRefVec<u8>) -> LinuxResult<RRefVec<u8>> {
        data.as_mut_slice().fill(0);
        Ok(data)
    }
    fn write(&self, data: &RRefVec<u8>) -> LinuxResult<usize> {
        Ok(data.len())
    }
}


pub fn main() -> Box<dyn EmptyDeviceDomain> {
    Box::new(NullDeviceDomainImpl)
}
