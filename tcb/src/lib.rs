#![no_std]

extern crate alloc;

use alloc::{borrow::ToOwned, string::String};

use linux_kernel_module::println;

struct TcbModule {
    message: String,
}

impl linux_kernel_module::KernelModule for TcbModule {
    fn init() -> linux_kernel_module::KernelResult<Self> {
        println!("TCB kernel module!");
        Ok(TcbModule {
            message: "on the heap!".to_owned(),
        })
    }
}

impl Drop for TcbModule {
    fn drop(&mut self) {
        println!("My message is {}", self.message);
        println!("Goodbye kernel module!");
    }
}

linux_kernel_module::kernel_module!(
    TcbModule,
    author: b"godones",
    description: b"TCB kernel module",
    license: b"GPL"
);
