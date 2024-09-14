#![no_std]

extern crate alloc;

use linux_kernel_module::{
    self, cstr, println, random,
    sysctl::{Sysctl, SysctlStorage},
    Mode,
};

struct EntropySource;

impl SysctlStorage for EntropySource {
    fn store_value(&self, data: &[u8]) -> (usize, linux_kernel_module::KernelResult<()>) {
        println!("EntropySource::store_value {:?}", data);
        random::add_randomness(data);
        (data.len(), Ok(()))
    }

    fn read_value(
        &self,
        data: &mut linux_kernel_module::kernel_ptr::KernelSlicePtrWriter,
    ) -> (usize, linux_kernel_module::KernelResult<()>) {
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

impl linux_kernel_module::KernelModule for RandomTestModule {
    fn init() -> linux_kernel_module::KernelResult<Self> {
        println!("RandomTestModule::init");
        Ok(RandomTestModule {
            _sysctl_entropy: Sysctl::register(
                cstr!("rust/rrandom"),
                cstr!("entropy"),
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

linux_kernel_module::kernel_module!(
    RandomTestModule,
    author: b"Fish in a Barrel Contributors",
    description: b"A module for testing the CSPRNG",
    license: b"GPL"
);
