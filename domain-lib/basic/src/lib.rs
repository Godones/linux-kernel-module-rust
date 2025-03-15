#![feature(downcast_unchecked)]
#![no_std]
extern crate alloc;

#[macro_use]
extern crate corelib;

pub mod logging;

use alloc::sync::Arc;

use corelib::domain_info::DomainInfo;
pub use corelib::{
    backtrace, bindings, blk_crash_trick, c_str, checkout_shared_data, impl_has_timer, kernel,
    new_device_data, new_mutex, new_spinlock, static_assert, sys_blk_mq_map_queues,
    sys_blk_mq_pci_map_queues, sys_create_domain, sys_dma_map_page_attrs, sys_dma_unmap_page_attrs,
    sys_get_domain, sys_mdelay, sys_num_possible_cpus, sys_register_domain, sys_reload_domain,
    sys_update_domain, sys_write_console, CoreFunction, LinuxError, LinuxResult, SafePtr,
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
    use core::mem::forget;
    let res = unwinding::panic::catch_unwind(f).unwrap_or_else(|r| {
        // println_color!(31, "[Panic] catch unwind error");
        let now = ktime_get_ns();
        let old = r.downcast_ref::<u64>().unwrap();
        println_color!(31, "[Panic] catch unwind error cost: {}ns", now - *old);
        forget(r);
        Err(LinuxError::DOMAINCRASH)
    });
    res
}

static mut GLOBAL_TIME: u64 = 0;

#[cfg(feature = "unwind")]
#[inline]
pub fn unwind_from_panic() {
    use alloc::boxed::Box;
    unwinding::panic::begin_panic(Box::new(0u64));
}

#[cfg(feature = "unwind")]
#[inline]
pub fn unwind_from_panic_with_time(time: u64) {
    use alloc::boxed::Box;
    let time = unsafe {
        GLOBAL_TIME = time;
        Box::from_raw(&raw mut GLOBAL_TIME)
    };
    unwinding::panic::begin_panic(time);
}

pub mod sync {
    pub use spin::Mutex;
}
use corelib::kernel::time::ktime_get_ns;
pub use corelib::{print, println, println_color};
