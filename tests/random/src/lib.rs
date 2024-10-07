#![no_std]

extern crate alloc;

use kernel::{
    self,
    buf::KernelSlicePtrWriter,
    c_str,
    error::KernelResult,
    module, println, random,
    sysctl::{Sysctl, SysctlStorage},
    types::Mode,
    Module, ThisModule,
};

struct EntropySource;

impl SysctlStorage for EntropySource {
    fn store_value(&self, data: &[u8]) -> (usize, KernelResult<()>) {
        println!("EntropySource::store_value {:?}", data);
        random::add_randomness(data);
        (data.len(), Ok(()))
    }

    fn read_value(&self, data: &mut KernelSlicePtrWriter) -> (usize, KernelResult<()>) {
        let mut storage = alloc::vec![0; data.len()];
        if let Err(e) = random::getrandom(&mut storage) {
            return (0, Err(e));
        }
        (storage.len(), data.write(&storage))
    }
}

struct RandomTestModule {
    _sysctl_entropy: Sysctl<EntropySource>,
}

impl Module for RandomTestModule {
    fn init(_module: &'static ThisModule) -> KernelResult<Self> {
        println!("RandomTestModule::init");
        Ok(RandomTestModule {
            _sysctl_entropy: Sysctl::register(
                c_str!("rust/rrandom"),
                c_str!("entropy"),
                EntropySource,
                Mode::from_int(0o666),
            )?,
        })
    }
}

impl Drop for RandomTestModule {
    fn drop(&mut self) {
        println!("RandomTestModule::drop");
    }
}

module! {
    type: RandomTestModule,
    name: "RandomTestModule",
    author: "Rust for Linux Contributors",
    description: "A module for testing the CSPRNG",
    license: "GPL",
}
