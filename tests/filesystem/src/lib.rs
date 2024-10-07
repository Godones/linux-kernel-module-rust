#![no_std]

extern crate alloc;

use kernel::{
    self, c_str,
    fs::filesystem::{self, FileSystem, FileSystemFlags},
    module,
    str::CStr,
    Module, ThisModule,
};

struct TestFSModule {
    _fs_registration: filesystem::Registration<TestFS>,
}

struct TestFS {}

impl FileSystem for TestFS {
    const NAME: &'static CStr = c_str!("testfs");
    const FLAGS: FileSystemFlags = FileSystemFlags::empty();
}

impl Module for TestFSModule {
    fn init(_module: &'static ThisModule) -> kernel::error::KernelResult<Self> {
        let fs_registration = filesystem::register::<TestFS>()?;
        Ok(TestFSModule {
            _fs_registration: fs_registration,
        })
    }
}

module! {
    type: TestFSModule,
    name: "TestFSModule",
    author: "Rust for Linux Contributors",
    description: "A module for testing filesystem::register",
    license: "GPL",
}
