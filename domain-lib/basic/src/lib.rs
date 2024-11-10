#![feature(downcast_unchecked)]
#![no_std]
extern crate alloc;

#[macro_use]
extern crate corelib;

pub mod logging;

use alloc::sync::Arc;

use corelib::domain_info::DomainInfo;
pub use corelib::{
    backtrace, blk_crash_trick, c_str, checkout_shared_data, create_domain, get_domain,
    impl_has_timer, kernel, new_mutex, new_spinlock, register_domain, reload_domain, static_assert,
    sys_blk_mq_map_queues, sys_blk_mq_pci_map_queues, sys_dma_map_page_attrs,
    sys_dma_unmap_page_attrs, update_domain, write_console, CoreFunction, LinuxError, LinuxResult,
    SafePtr,sys_num_possible_cpus,new_device_data,sys_mdelay,bindings
};
pub use domain_main::domain_main;
use ksync::Mutex;
pub type DomainInfoSet = Mutex<DomainInfo>;

pub fn domain_info() -> Arc<DomainInfoSet> {
    let res = corelib::domain_info().unwrap();
    unsafe { res.downcast_unchecked() }
}

#[cfg(feature = "unwind")]
pub fn catch_unwind<F: FnOnce() -> LinuxResult<R>, R>(f: F) -> LinuxResult<R> {
    let res = unwinding::panic::catch_unwind(f).unwrap_or_else(|_| {
        println_color!(31, "[Panic] catch unwind error");
        Err(LinuxError::DOMAINCRASH)
    });
    res
}

#[cfg(feature = "unwind")]
#[inline]
pub fn unwind_from_panic() {
    use alloc::boxed::Box;
    unwinding::panic::begin_panic(Box::new(()));
}

pub mod sync {
    pub use spin::Mutex;
}

#[macro_use]
pub mod console {
    pub use corelib::{print, println, println_color};
}
