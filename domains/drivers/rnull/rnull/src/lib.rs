#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(allocator_api)]
mod block_domain;

extern crate alloc;
use alloc::boxed::Box;
use core::fmt::Debug;

use basic::{kernel::block::mq::OperationsConverter, println, LinuxError, LinuxResult};
use interface::{
    null_block::{BlockArgs, BlockDeviceDomain},
    Basic,
};
use spin::Mutex;

use crate::block_domain::{NullBlkDevice, NullBlkDomain};

#[derive(Debug)]
struct NullDeviceDomainImpl {
    block: Mutex<Option<NullBlkDomain>>,
}

impl NullDeviceDomainImpl {
    pub fn new() -> Self {
        Self {
            block: Mutex::new(None),
        }
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
        let block = NullBlkDomain::init(args).map_err(|e| {
            println!("NullBlkModule init error: {:?}", e);
            LinuxError::EINVAL
        })?;
        *self.block.lock() = Some(block);
        Ok(())
    }
    fn tag_set_with_queue_data(&self) -> LinuxResult<(usize, usize)> {
        let blk = self.block.lock();
        let blk = blk.as_ref().ok_or(LinuxError::EINVAL)?;
        let res = blk.tag_set_with_queue_data();
        match res {
            Ok(r) => Ok(r),
            Err(e) => {
                println!("NullBlkModule tag_set_with_queue_data error: {:?}", e);
                Err(LinuxError::EINVAL)
            }
        }
    }

    fn set_gen_disk(&self, gen_disk: usize) -> LinuxResult<usize> {
        let blk = self.block.lock();
        let blk = blk.as_ref().ok_or(LinuxError::EINVAL)?;
        blk.set_gen_disk(gen_disk).map_err(|e| {
            println!("NullBlkModule set_gen_disk error: {:?}", e);
            LinuxError::EINVAL
        })
    }

    fn open(&self, _mode: u32) -> LinuxResult<()> {
        Ok(())
    }

    fn release(&self) -> LinuxResult<()> {
        Ok(())
    }

    fn init_request(
        &self,
        tag_set_ptr: usize,
        rq_ptr: usize,
        driver_data_ptr: usize,
    ) -> LinuxResult<()> {
        OperationsConverter::<NullBlkDevice>::init_request(tag_set_ptr, rq_ptr, driver_data_ptr)
            .map_err(|e| {
                println!("NullBlkModule init_request error: {:?}", e);
                LinuxError::EINVAL
            })
    }

    fn exit_request(&self, tag_set_ptr: usize, rq_ptr: usize) -> LinuxResult<()> {
        OperationsConverter::<NullBlkDevice>::exit_request(tag_set_ptr, rq_ptr).map_err(|e| {
            println!("NullBlkModule exit_request error: {:?}", e);
            LinuxError::EINVAL
        })
    }

    fn init_hctx(
        &self,
        hctx_ptr: usize,
        tag_set_data_ptr: usize,
        hctx_idx: usize,
    ) -> LinuxResult<()> {
        OperationsConverter::<NullBlkDevice>::init_hctx(hctx_ptr, tag_set_data_ptr, hctx_idx)
            .map_err(|e| {
                println!("NullBlkModule init_hctx error: {:?}", e);
                LinuxError::EINVAL
            })
    }

    fn exit_hctx(&self, hctx_ptr: usize, hctx_idx: usize) -> LinuxResult<()> {
        OperationsConverter::<NullBlkDevice>::exit_hctx(hctx_ptr, hctx_idx).map_err(|e| {
            println!("NullBlkModule exit_hctx error: {:?}", e);
            LinuxError::EINVAL
        })
    }

    fn queue_rq(
        &self,
        hctx_ptr: usize,
        bd_ptr: usize,
        hctx_driver_data_ptr: usize,
    ) -> LinuxResult<()> {
        OperationsConverter::<NullBlkDevice>::queue_rq(hctx_ptr, bd_ptr, hctx_driver_data_ptr)
            .map_err(|e| {
                println!("NullBlkModule queue_rq error: {:?}", e);
                LinuxError::EINVAL
            })
    }

    fn exit(&self) -> LinuxResult<()> {
        let v = self.block.lock().take();
        drop(v);
        println!("NullDeviceDomainImpl exit");
        Ok(())
    }
}

pub fn main() -> Box<dyn BlockDeviceDomain> {
    Box::new(NullDeviceDomainImpl::new())
}
