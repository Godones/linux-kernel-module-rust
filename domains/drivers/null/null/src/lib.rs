#![no_std]
#![forbid(unsafe_code)]
extern crate alloc;

use alloc::boxed::Box;
use core::fmt::Debug;

use basic::{kernel::time::ktime_get_ns, println, LinuxResult};
use interface::{empty_device::EmptyDeviceDomain, Basic};
use shared_heap::{DBox, DVec};
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

    pub fn do_read(&self, mut data: DVec<u8>) -> LinuxResult<DVec<u8>> {
        let now = ktime_get_ns();
        data.as_mut_slice().fill(now as u8);
        // wait 10 ms
        // loop {
        //     let n = ktime_get_ns();
        //     if n - now > 10_000_000 {
        //         break;
        //     }
        //     data.as_mut_slice()[n as usize%100] = 0;
        // }
        Ok(data)
    }

    pub fn do_write(&self, data: &DVec<u8>) -> LinuxResult<usize> {
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
        shared_heap::domain_id()
    }
}

impl EmptyDeviceDomain for NullDeviceDomainImpl {
    fn init(&self) -> LinuxResult<()> {
        // println!("NullDeviceDomainImpl init");
        Ok(())
    }

    fn read(&self, data: DVec<u8>) -> LinuxResult<DVec<u8>> {
        self.do_read(data)
    }
    fn write(&self, data: &DVec<u8>) -> LinuxResult<usize> {
        // println!("NullDeviceDomainImpl write");
        self.do_write(data)
    }

    fn no_arg(&self) -> LinuxResult<()>{
        Ok(())
    }
    fn one_arg(&self, arg: u64) -> LinuxResult<u64>{
        Ok(arg+1)
    }
    fn one_darg(&self, arg: DBox<usize>) -> LinuxResult<DBox<usize>>{
        let mut arg = arg;
        *arg +=1;
        Ok(arg)
    }
    fn two_dargs(&self, arg1: DBox<usize>, arg2: DBox<usize>) -> LinuxResult<(DBox<usize>,DBox<usize>)>{
        let mut arg1 = arg1;
        let mut arg2 = arg2;
        *arg1 += 1;
        *arg2 += 2;
        Ok((arg1,arg2))
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
    fn read(&self, data: DVec<u8>) -> LinuxResult<DVec<u8>> {
        basic::catch_unwind(|| self.0.read(data))
    }
    fn write(&self, data: &DVec<u8>) -> LinuxResult<usize> {
        basic::catch_unwind(|| self.0.write(data))
    }
    fn no_arg(&self) -> LinuxResult<()>{
        basic::catch_unwind(|| self.0.no_arg())
    }
    fn one_arg(&self, arg: u64) -> LinuxResult<u64>{
        basic::catch_unwind(|| self.0.one_arg(arg))
    }
    fn one_darg(&self, arg: DBox<usize>) -> LinuxResult<DBox<usize>>{
        basic::catch_unwind(|| self.0.one_darg(arg))
    }
    fn two_dargs(&self, arg1: DBox<usize>, arg2: DBox<usize>) -> LinuxResult<(DBox<usize>,DBox<usize>)>{
        basic::catch_unwind(|| self.0.two_dargs(arg1,arg2))
    }
}

pub fn main() -> Box<dyn EmptyDeviceDomain> {
    Box::new(UnwindWrap::new(NullDeviceDomainImpl::new()))
}
