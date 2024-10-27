#![no_std]
#![feature(impl_trait_in_assoc_type)]
#![feature(allocator_api)]
mod block_domain;

extern crate alloc;
use alloc::boxed::Box;
use core::fmt::Debug;

use basic::{kernel::block::mq::OperationsConverter, println, LinuxError, LinuxResult, SafePtr};
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
    fn tag_set_with_queue_data(&self) -> LinuxResult<(SafePtr, SafePtr)> {
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

    fn set_gen_disk(&self, gen_disk: SafePtr) -> LinuxResult<()> {
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

    fn init_request(&self, tag_set_ptr: SafePtr, rq_ptr: SafePtr, driver_data_ptr: SafePtr) -> LinuxResult<()> {
        OperationsConverter::<NullBlkDevice>::init_request(tag_set_ptr, rq_ptr, driver_data_ptr)
            .map_err(|e| {
                println!("NullBlkModule init_request error: {:?}", e);
                LinuxError::EINVAL
            })
    }

    fn exit_request(&self, tag_set_ptr: SafePtr, rq_ptr: SafePtr) -> LinuxResult<()> {
        OperationsConverter::<NullBlkDevice>::exit_request(tag_set_ptr, rq_ptr).map_err(|e| {
            println!("NullBlkModule exit_request error: {:?}", e);
            LinuxError::EINVAL
        })
    }

    fn init_hctx(&self, hctx_ptr: SafePtr, tag_set_data_ptr: SafePtr, hctx_idx: usize) -> LinuxResult<()> {
        OperationsConverter::<NullBlkDevice>::init_hctx(hctx_ptr, tag_set_data_ptr, hctx_idx)
            .map_err(|e| {
                println!("NullBlkModule init_hctx error: {:?}", e);
                LinuxError::EINVAL
            })
    }

    fn exit_hctx(&self, hctx_ptr: SafePtr, hctx_idx: usize) -> LinuxResult<()> {
        OperationsConverter::<NullBlkDevice>::exit_hctx(hctx_ptr, hctx_idx).map_err(|e| {
            println!("NullBlkModule exit_hctx error: {:?}", e);
            LinuxError::EINVAL
        })
    }

    fn queue_rq(&self, hctx_ptr: SafePtr, bd_ptr: SafePtr, hctx_driver_data_ptr: SafePtr) -> LinuxResult<()> {
        OperationsConverter::<NullBlkDevice>::queue_rq(hctx_ptr, bd_ptr, hctx_driver_data_ptr)
            .map_err(|e| {
                println!("NullBlkModule queue_rq error: {:?}", e);
                LinuxError::EINVAL
            })
    }
    fn commit_rqs(&self, hctx_ptr: SafePtr, hctx_driver_data_ptr: SafePtr) -> LinuxResult<()> {
        OperationsConverter::<NullBlkDevice>::commit_rqs(hctx_ptr, hctx_driver_data_ptr).map_err(
            |e| {
                println!("NullBlkModule commit_rqs error: {:?}", e);
                LinuxError::EINVAL
            },
        )
    }
    fn complete_request(&self, rq_ptr: SafePtr) -> LinuxResult<()> {
        OperationsConverter::<NullBlkDevice>::complete_request(rq_ptr).map_err(|e| {
            println!("NullBlkModule complete_request error: {:?}", e);
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

#[derive(Debug)]
pub struct UnwindWrap(NullDeviceDomainImpl);

impl Basic for UnwindWrap {
    fn domain_id(&self) -> u64 {
        self.0.domain_id()
    }
}

impl BlockDeviceDomain for UnwindWrap{
    fn init(&self, args: &BlockArgs) -> LinuxResult<()> {
        self.0.init(args)
    }

    fn tag_set_with_queue_data(&self) -> LinuxResult<(SafePtr, SafePtr)> {
        basic::catch_unwind(|| self.0.tag_set_with_queue_data())
    }

    fn set_gen_disk(&self, gen_disk: SafePtr) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.set_gen_disk(gen_disk))
    }

    fn open(&self, mode: u32) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.open(mode))
    }

    fn release(&self) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.release())
    }

    fn init_request(&self, tag_set_ptr: SafePtr, rq_ptr: SafePtr, driver_data_ptr: SafePtr) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.init_request(tag_set_ptr, rq_ptr, driver_data_ptr))
    }

    fn exit_request(&self, tag_set_ptr: SafePtr, rq_ptr: SafePtr) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.exit_request(tag_set_ptr, rq_ptr))
    }

    fn init_hctx(&self, hctx_ptr: SafePtr, tag_set_data_ptr: SafePtr, hctx_idx: usize) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.init_hctx(hctx_ptr, tag_set_data_ptr, hctx_idx))
    }

    fn exit_hctx(&self, hctx_ptr: SafePtr, hctx_idx: usize) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.exit_hctx(hctx_ptr, hctx_idx))
    }

    fn queue_rq(&self, hctx_ptr: SafePtr, bd_ptr: SafePtr, hctx_driver_data_ptr: SafePtr) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.queue_rq(hctx_ptr, bd_ptr, hctx_driver_data_ptr))
    }

    fn commit_rqs(&self, hctx_ptr: SafePtr, hctx_driver_data_ptr: SafePtr) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.commit_rqs(hctx_ptr, hctx_driver_data_ptr))
    }

    fn complete_request(&self, rq_ptr: SafePtr) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.complete_request(rq_ptr))
    }

    fn exit(&self) -> LinuxResult<()> {
        basic::catch_unwind(|| self.0.exit())
    }
}




pub fn main() -> Box<dyn BlockDeviceDomain> {
    Box::new(UnwindWrap(NullDeviceDomainImpl::new()))
}
