#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![feature(let_chains)]
#![no_std]

extern crate alloc;
#[macro_use]
extern crate log;

#[macro_use]
extern crate linux_kernel_module;

mod config;
mod domain;
mod domain_helper;
mod domain_loader;
mod domain_proxy;
mod mem;

use alloc::{borrow::ToOwned, string::String};

use linux_kernel_module::{logger, println};

struct TcbModule {
    message: String,
}

impl linux_kernel_module::KernelModule for TcbModule {
    fn init() -> linux_kernel_module::KernelResult<Self> {
        println!("TCB kernel module!");
        println_color!(31, "This is a red message");
        println_color!(32, "This is a green message");
        println_color!(33, "This is a yellow message");
        logger::init_logger();
        domain::init_domain_system().unwrap();
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
