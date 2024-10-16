#![feature(downcast_unchecked)]
#![no_std]
extern crate alloc;

#[macro_use]
pub mod console;
pub mod logging;

use alloc::sync::Arc;

use corelib::domain_info::DomainInfo;
pub use corelib::{
    backtrace, blk_crash_trick, checkout_shared_data, create_domain, get_domain, impl_has_timer,
    kernel, new_mutex, new_spinlock, register_domain, reload_domain, update_domain, write_console,
    CoreFunction, LinuxError, LinuxResult,
};
use ksync::Mutex;

pub type DomainInfoSet = Mutex<DomainInfo>;

pub fn domain_info() -> Arc<DomainInfoSet> {
    let res = corelib::domain_info().unwrap();
    unsafe { res.downcast_unchecked() }
}
