#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![feature(let_chains)]
#![feature(box_into_inner)]
#![no_std]

extern crate alloc;
#[macro_use]
extern crate log;

#[macro_use]
extern crate kbind;

mod channel;
mod config;
mod domain;
mod domain_helper;
mod domain_loader;
mod domain_proxy;
mod mem;

use alloc::{borrow::ToOwned, string::String};

use kbind::{logger, println, sysctl::Sysctl};

use crate::channel::CommandChannel;

struct TcbModule {
    _sysctl_domain_command: Sysctl<CommandChannel>,
    message: String,
}

impl kbind::KernelModule for TcbModule {
    fn init() -> kbind::KernelResult<Self> {
        println!("TCB kernel module!");
        println_color!(31, "This is a red message");
        println_color!(32, "This is a green message");
        println_color!(33, "This is a yellow message");
        logger::init_logger();
        let channel = channel::init_domain_channel()?;
        domain::init_domain_system().unwrap();
        Ok(TcbModule {
            _sysctl_domain_command: channel,
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

kbind::kernel_module!(
    TcbModule,
    author: b"godones",
    description: b"TCB kernel module",
    license: b"GPL"
);
