#![no_std]

extern crate alloc;

use alloc::string::ToString;
use core::sync::atomic::{AtomicUsize, Ordering};

use kernel::{
    buf::{UserSlicePtrReader, UserSlicePtrWriter},
    c_str, chrdev,
    error::KernelResult,
    fs, module, Module, ThisModule,
};

struct CycleFile;

impl fs::file_operations::FileOperations for CycleFile {
    fn open() -> KernelResult<Self> {
        Ok(CycleFile)
    }

    const READ: fs::file_operations::ReadFn<Self> = Some(
        |_this: &Self,
         _file: &fs::file_operations::File,
         buf: &mut UserSlicePtrWriter,
         offset: u64|
         -> KernelResult<()> {
            for c in b"123456789"
                .iter()
                .cycle()
                .skip((offset % 9) as _)
                .take(buf.len())
            {
                buf.write(&[*c])?;
            }
            Ok(())
        },
    );
}

struct SeekFile;

impl fs::file_operations::FileOperations for SeekFile {
    fn open() -> KernelResult<Self> {
        Ok(SeekFile)
    }

    const SEEK: fs::file_operations::SeekFn<Self> = Some(
        |_this: &Self,
         _file: &fs::file_operations::File,
         _offset: fs::file_operations::SeekFrom|
         -> KernelResult<u64> { Ok(1234) },
    );
}

struct WriteFile {
    written: AtomicUsize,
}

impl fs::file_operations::FileOperations for WriteFile {
    fn open() -> KernelResult<Self> {
        Ok(WriteFile {
            written: AtomicUsize::new(0),
        })
    }

    const READ: fs::file_operations::ReadFn<Self> = Some(
        |this: &Self,
         _file: &fs::file_operations::File,
         buf: &mut UserSlicePtrWriter,
         _offset: u64|
         -> KernelResult<()> {
            let val = this.written.load(Ordering::SeqCst).to_string();
            buf.write(val.as_bytes())?;
            Ok(())
        },
    );

    const WRITE: fs::file_operations::WriteFn<Self> = Some(
        |this: &Self, buf: &mut UserSlicePtrReader, _offset: u64| -> KernelResult<()> {
            let data = buf.read_all()?;
            this.written.fetch_add(data.len(), Ordering::SeqCst);
            Ok(())
        },
    );
}

struct ChrdevTestModule {
    _chrdev_registration: chrdev::Registration,
}

impl Module for ChrdevTestModule {
    fn init(_module: &'static ThisModule) -> KernelResult<Self> {
        let chrdev_registration = chrdev::builder(c_str!("chrdev-tests"), 0..3)?
            .register_device::<CycleFile>()
            .register_device::<SeekFile>()
            .register_device::<WriteFile>()
            .build()?;
        Ok(ChrdevTestModule {
            _chrdev_registration: chrdev_registration,
        })
    }
}

module! {
    type: ChrdevTestModule,
    name: "ChrdevTestModule",
    author: "Rust for Linux Contributors",
    description: "A module for testing character devices",
    license: "GPL",
}
