#![no_std]

use kernel::{c_str, chrdev, module, Module, ThisModule};

struct ChrdevRegionAllocationTestModule {
    _chrdev_reg: chrdev::Registration,
}

impl Module for ChrdevRegionAllocationTestModule {
    fn init(_module: &'static ThisModule) -> kernel::error::KernelResult<Self> {
        let chrdev_reg =
            chrdev::builder(c_str!("chrdev-region-allocation-tests"), 0..1)?.build()?;

        Ok(ChrdevRegionAllocationTestModule {
            _chrdev_reg: chrdev_reg,
        })
    }
}

module! {
    type: ChrdevRegionAllocationTestModule,
    name: "ChrdevRegionAllocationTestModule",
    author: "Rust for Linux Contributors",
    description: "A module for testing character device region allocation",
    license: "GPL",
}
