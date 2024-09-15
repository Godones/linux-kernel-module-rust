#![feature(downcast_unchecked)]
#![no_std]

#[macro_use]
pub mod console;
pub mod logging;

pub use corelib::{
    backtrace, blk_crash_trick, checkout_shared_data, create_domain, get_domain, register_domain,
    reload_domain, update_domain, write_console, CoreFunction, LinuxError, LinuxResult,
};
