#![no_std]

use core::sync::atomic::AtomicBool;

use kernel::{c_str, module, sysctl::Sysctl, types::Mode, Module, ThisModule};

struct SysctlTestModule {
    _sysctl_a: Sysctl<AtomicBool>,
    _sysctl_b: Sysctl<AtomicBool>,
}

impl Module for SysctlTestModule {
    fn init(_module: &'static ThisModule) -> kernel::error::KernelResult<Self> {
        Ok(SysctlTestModule {
            _sysctl_a: Sysctl::register(
                c_str!("rust/sysctl-tests"),
                c_str!("a"),
                AtomicBool::new(false),
                Mode::from_int(0o666),
            )?,
            _sysctl_b: Sysctl::register(
                c_str!("rust/sysctl-tests"),
                c_str!("b"),
                AtomicBool::new(false),
                Mode::from_int(0o666),
            )?,
        })
    }
}

module! {
    type: SysctlTestModule,
    name: "SysctlTestModule",
    author: "Rust for Linux Contributors",
    description: "A module for testing sysctls",
    license: "GPL",
}
