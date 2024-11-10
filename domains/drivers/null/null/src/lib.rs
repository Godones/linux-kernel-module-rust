#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;

use alloc::{boxed::Box, string::String};
use core::{fmt::Debug, sync::atomic::AtomicBool};
use basic::{LinuxResult};
use basic::console::*;
use interface::{empty_device::EmptyDeviceDomain, Basic};
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
        data.as_mut_slice().fill(1);
        Ok(data)
    }
    fn write(&self, data: &RRefVec<u8>) -> LinuxResult<usize> {
        static FLAG: AtomicBool = AtomicBool::new(true);
        if FLAG.load(core::sync::atomic::Ordering::Relaxed) {
            println!("NullDeviceDomainImpl::read: panic test");
            FLAG.store(false, core::sync::atomic::Ordering::Relaxed);
            bar();
        }
        Ok(data.len())
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
    Box::new(UnwindWrap::new(NullDeviceDomainImpl))
}

#[derive(Debug)]
struct PrintOnDrop(String);

impl Drop for PrintOnDrop {
    fn drop(&mut self) {
        println!("dropped: {:?}", self.0);
    }
}

fn foo() {
    panic!("panic at foo\n");
}

#[inline(never)]
fn bar() {
    use alloc::string::String;
    let p1 = PrintOnDrop(String::from("PrintOnDrop1"));
    let p2 = PrintOnDrop(String::from("PrintOnDrop2"));
    println!("p1: {:?}, p2: {:?}", p1, p2);
    foo()
}
