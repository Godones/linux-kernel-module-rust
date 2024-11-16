#![feature(allocator_api)]
#![feature(alloc_layout_extra)]
#![feature(let_chains)]
#![feature(box_into_inner)]
#![no_std]

extern crate alloc;
#[macro_use]
extern crate log;

#[macro_use]
extern crate kernel;

mod channel;
mod config;
mod domain;
mod domain_helper;
mod domain_loader;
mod domain_proxy;
mod kshim;
mod mem;

use alloc::{borrow::ToOwned, string::String};

use kernel::{code, sysctl::Sysctl, ThisModule};
use spin::Once;

use crate::{channel::CommandChannel, kshim::KObj};

struct TcbModule {
    _sysctl_domain_command: Sysctl<CommandChannel>,
    kobj: KObj,
    message: String,
}

static MODULE: Once<&'static ThisModule> = Once::new();

impl kernel::Module for TcbModule {
    fn init(module: &'static ThisModule) -> kernel::error::KernelResult<Self> {
        println!("TCB kernel module!");
        println_color!(31, "This is a red message");
        println_color!(32, "This is a green message");
        println_color!(33, "This is a yellow message");
        kernel::logger::init_logger();
        MODULE.call_once(|| module);
        let channel = channel::init_domain_channel()?;
        domain::init_domain_system().map_err(|e| {
            error!("Failed to init domain system: {:?}", e);
            code::EINVAL
        })?;
        let kobj = kshim::init_kernel_shim()?;
        Ok(TcbModule {
            _sysctl_domain_command: channel,
            kobj,
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

module! {
    type: TcbModule,
    name: "TcbModule",
    author: "godones",
    description: "TCB kernel module",
    license: "GPL",
}
