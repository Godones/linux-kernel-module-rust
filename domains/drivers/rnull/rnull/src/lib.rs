#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(allocator_api)]

mod block;

extern crate alloc;

use alloc::boxed::Box;
use core::fmt::Debug;
use spin::Mutex;
use basic::{println, LinuxError, LinuxResult};
use interface::{null_block::BlockDeviceDomain, Basic};
use interface::null_block::BlockArgs;
use crate::block::NullBlkModule;


struct NullDeviceDomainImpl{
    block:Mutex<Option<NullBlkModule>>
}

impl NullDeviceDomainImpl{
    pub fn new() -> Self {
        Self {
            block: Mutex::new(None)
        }
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

impl BlockDeviceDomain for NullDeviceDomainImpl {
    fn init(&self, args: &BlockArgs) -> LinuxResult<()> {
        println!("NullDeviceDomainImpl init");
        println!("args: {:?}", args);
        let block = NullBlkModule::init(args)
            .map_err(|e| {
                println!("NullBlkModule init error: {:?}", e);
                LinuxError::EINVAL
            })?;
        *self.block.lock() = Some(block);
        Ok(())
    }

    fn exit(&self) -> LinuxResult<()> {
        println!("NullDeviceDomainImpl exit");
        self.block.lock().take();
        Ok(())
    }
}

pub fn main() -> Box<dyn BlockDeviceDomain> {
    Box::new(NullDeviceDomainImpl::new())
}
