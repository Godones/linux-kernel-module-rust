#![no_std]
extern crate alloc;

use alloc::sync::Arc;
use core::any::Any;

#[cfg(feature = "core_impl")]
pub use core_impl::*;
use interface::{DomainType, DomainTypeRaw};
pub use pconst::LinuxErrno;
use spin::Once;

pub mod domain_info;
pub type LinuxResult<T> = Result<T, LinuxErrno>;
pub type LinuxError = LinuxErrno;

pub trait CoreFunction: Send + Sync {
    fn sys_alloc_pages(&self, domain_id: u64, n: usize) -> *mut u8;
    fn sys_free_pages(&self, domain_id: u64, p: *mut u8, n: usize);
    fn sys_write_console(&self, s: &str);
    fn sys_backtrace(&self, domain_id: u64);
    /// This func will be deleted
    fn blk_crash_trick(&self) -> bool;
    fn sys_get_domain(&self, name: &str) -> Option<DomainType>;
    fn sys_create_domain(
        &self,
        domain_file_name: &str,
        identifier: &mut [u8],
    ) -> LinuxResult<DomainType>;
    /// Register a new domain with the given name and type
    fn sys_register_domain(&self, ident: &str, ty: DomainTypeRaw, data: &[u8]) -> LinuxResult<()>;
    /// Replace the old domain with the new domain
    fn sys_update_domain(
        &self,
        old_domain_name: &str,
        new_domain_name: &str,
        ty: DomainTypeRaw,
    ) -> LinuxResult<()>;
    fn sys_reload_domain(&self, domain_name: &str) -> LinuxResult<()>;
    fn checkout_shared_data(&self) -> LinuxResult<()>;
    fn domain_info(&self) -> LinuxResult<Arc<dyn Any + Send + Sync>>;
}

#[cfg(feature = "core_impl")]
mod core_impl {
    use alloc::sync::Arc;
    use core::any::Any;

    use interface::{DomainType, DomainTypeRaw};
    use spin::Once;

    use super::{LinuxError, LinuxResult, OnceGet};
    use crate::CoreFunction;

    static CORE_FUNC: Once<&'static dyn CoreFunction> = Once::new();

    extern "C" {
        fn sbss();
        fn ebss();
    }
    fn clear_bss() {
        unsafe {
            core::slice::from_raw_parts_mut(
                sbss as usize as *mut u8,
                ebss as usize - sbss as usize,
            )
            .fill(0);
        }
    }

    pub fn init(syscall: &'static dyn CoreFunction) {
        clear_bss();
        CORE_FUNC.call_once(|| syscall);
    }

    pub fn alloc_raw_pages(n: usize, domain_id: u64) -> *mut u8 {
        CORE_FUNC.get_must().sys_alloc_pages(domain_id, n)
    }

    pub fn free_raw_pages(p: *mut u8, n: usize, domain_id: u64) {
        CORE_FUNC.get_must().sys_free_pages(domain_id, p, n);
    }

    pub fn write_console(s: &str) {
        CORE_FUNC.get_must().sys_write_console(s);
    }

    pub fn backtrace(domain_id: u64) {
        CORE_FUNC.get_must().sys_backtrace(domain_id);
    }

    // todo!(delete)
    pub fn blk_crash_trick() -> bool {
        CORE_FUNC.get_must().blk_crash_trick()
    }

    pub fn get_domain(name: &str) -> Option<DomainType> {
        CORE_FUNC.get_must().sys_get_domain(name)
    }

    pub fn create_domain(
        domain_file_name: &str,
        domain_identifier: &mut [u8],
    ) -> LinuxResult<DomainType> {
        if domain_identifier.len() < 32 {
            return Err(LinuxError::EINVAL);
        }
        CORE_FUNC
            .get_must()
            .sys_create_domain(domain_file_name, domain_identifier)
    }

    pub fn register_domain(ident: &str, ty: DomainTypeRaw, data: &[u8]) -> LinuxResult<()> {
        CORE_FUNC.get_must().sys_register_domain(ident, ty, data)
    }

    pub fn update_domain(
        old_domain_name: &str,
        new_domain_name: &str,
        ty: DomainTypeRaw,
    ) -> LinuxResult<()> {
        CORE_FUNC
            .get_must()
            .sys_update_domain(old_domain_name, new_domain_name, ty)
    }

    pub fn reload_domain(domain_name: &str) -> LinuxResult<()> {
        CORE_FUNC.get_must().sys_reload_domain(domain_name)
    }
    pub fn checkout_shared_data() -> LinuxResult<()> {
        CORE_FUNC.get_must().checkout_shared_data()
    }

    pub fn domain_info() -> LinuxResult<Arc<dyn Any + Send + Sync>> {
        CORE_FUNC.get_must().domain_info()
    }
}

impl<T> OnceGet<T> for Once<T> {
    fn get_must(&self) -> &T {
        unsafe { self.get_unchecked() }
    }
}

pub trait OnceGet<T> {
    fn get_must(&self) -> &T;
}
