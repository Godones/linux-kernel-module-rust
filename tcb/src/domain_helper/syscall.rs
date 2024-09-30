use alloc::{string::ToString, sync::Arc};
use core::{any::Any, sync::atomic::AtomicBool};

use corelib::{domain_info::DomainDataInfo, CoreFunction, LinuxError, LinuxResult};
use interface::*;

use crate::{
    config::FRAME_BITS,
    domain_helper::{resource::DOMAIN_RESOURCE, DOMAIN_CREATE, DOMAIN_INFO},
    domain_proxy::{empty_device::EmptyDeviceDomainProxy, logger::LogDomainProxy},
};

pub static DOMAIN_SYS: &'static dyn CoreFunction = &DomainSyscall;

pub struct DomainSyscall;

impl CoreFunction for DomainSyscall {
    fn sys_alloc_pages(&self, domain_id: u64, n: usize) -> *mut u8 {
        let n = n.next_power_of_two();
        let page = crate::mem::alloc_frames(n);
        // info!(
        //     "[Domain: {}] alloc pages: {}, range:[{:#x}-{:#x}]",
        //     domain_id,
        //     n,
        //     page as usize,
        //     page as usize + n * FRAME_SIZE
        // );
        DOMAIN_RESOURCE
            .lock()
            .insert_page_map(domain_id, (page as usize >> FRAME_BITS, n));
        page
    }

    fn sys_free_pages(&self, domain_id: u64, p: *mut u8, n: usize) {
        let n = n.next_power_of_two();
        debug!("[Domain: {}] free pages: {}, ptr: {:p}", domain_id, n, p);
        DOMAIN_RESOURCE
            .lock()
            .free_page_map(domain_id, p as usize >> FRAME_BITS);
        crate::mem::free_frames(p, n);
    }

    fn sys_write_console(&self, s: &str) {
        print_raw!("{}", s);
    }

    fn sys_backtrace(&self, domain_id: u64) {
        let mut info = DOMAIN_INFO.lock();
        info.domain_list
            .get_mut(&domain_id)
            .map(|d| d.panic_count += 1);
        unwind();
    }

    fn blk_crash_trick(&self) -> bool {
        BLK_CRASH.load(core::sync::atomic::Ordering::Relaxed)
    }

    fn sys_get_domain(&self, name: &str) -> Option<DomainType> {
        super::query_domain(name)
    }

    fn sys_create_domain(
        &self,
        domain_file_name: &str,
        identifier: &mut [u8],
    ) -> LinuxResult<DomainType> {
        DOMAIN_CREATE
            .get()
            .unwrap()
            .create_domain(domain_file_name, identifier)
    }

    fn sys_register_domain(&self, ident: &str, ty: DomainTypeRaw, data: &[u8]) -> LinuxResult<()> {
        crate::domain_loader::creator::register_domain_elf(ident, data.to_vec(), ty);
        Ok(())
    }

    fn sys_update_domain(
        &self,
        old_domain_name: &str,
        new_domain_name: &str,
        ty: DomainTypeRaw,
    ) -> LinuxResult<()> {
        let old_domain = super::query_domain(old_domain_name);
        let old_domain_id = old_domain.as_ref().map(|d| d.domain_id());
        let (domain_info, new_domain_id) = match old_domain {
            Some(DomainType::LogDomain(logger)) => {
                let old_domain_id = logger.domain_id();
                let (id, new_domain, loader) = crate::domain_loader::creator::create_domain(
                    ty,
                    new_domain_name,
                    None,
                    Some(old_domain_id),
                )
                .ok_or(LinuxError::EINVAL)?;
                let logger_proxy = logger.downcast_arc::<LogDomainProxy>().unwrap();
                let domain_info = loader.domain_file_info();
                logger_proxy.replace(new_domain, loader)?;
                println!(
                    "Try to replace logger domain {} with {} ok",
                    old_domain_name, new_domain_name
                );
                Ok((domain_info, id))
            }
            Some(DomainType::EmptyDeviceDomain(empty_device)) => {
                let old_domain_id = empty_device.domain_id();
                let (id, new_domain, loader) = crate::domain_loader::creator::create_domain(
                    ty,
                    new_domain_name,
                    None,
                    Some(old_domain_id),
                )
                .ok_or(LinuxError::EINVAL)?;
                let empty_device = empty_device
                    .downcast_arc::<EmptyDeviceDomainProxy>()
                    .unwrap();
                let domain_info = loader.domain_file_info();
                empty_device.replace(new_domain, loader)?;
                println!(
                    "Try to replace empty device domain {} with {} ok",
                    old_domain_name, new_domain_name
                );
                Ok((domain_info, id))
            }
            None => {
                println!(
                    "<sys_update_domain> old domain {:?} not found",
                    old_domain_name
                );
                Err(LinuxError::EINVAL)
            } // Some(d) => {
              //     pr_err!("replace domain not support: {:?}", d);
              //     Err(LinuxError::EINVAL)
              // }
        }?;
        let domain_data = DomainDataInfo {
            name: old_domain_name.to_string(),
            ty,
            panic_count: 0,
            file_info: domain_info,
        };

        let mut info = DOMAIN_INFO.lock();
        info.domain_list.remove(&old_domain_id.unwrap());
        info.domain_list.insert(new_domain_id, domain_data);
        Ok(())
    }
    fn sys_reload_domain(&self, domain_name: &str) -> LinuxResult<()> {
        let domain = super::query_domain(domain_name).ok_or(LinuxError::EINVAL)?;
        match domain {
            // todo!(release old domain's resource)
            ty => {
                panic!("reload domain {:?} not support", ty);
            }
        }
    }

    fn checkout_shared_data(&self) -> LinuxResult<()> {
        crate::domain_helper::checkout_shared_data();
        Ok(())
    }

    fn domain_info(&self) -> LinuxResult<Arc<dyn Any + Send + Sync>> {
        let info = DOMAIN_INFO.clone();
        Ok(info)
    }
}

static BLK_CRASH: AtomicBool = AtomicBool::new(true);
fn unwind() {
    BLK_CRASH.store(false, core::sync::atomic::Ordering::Relaxed);
}
