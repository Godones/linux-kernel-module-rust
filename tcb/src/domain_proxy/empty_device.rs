use alloc::boxed::Box;
use core::{any::Any, mem::forget, pin::Pin, sync::atomic::AtomicBool};

use corelib::{LinuxError, LinuxResult};
use interface::{empty_device::EmptyDeviceDomain, Basic};
use kernel::{
    init::InPlaceInit,
    sync::{
        local_irq_restore, local_irq_save, switch_task_to_cpus, sync_cpus, LongLongPerCpu, Mutex,
        SRcuData,
    },
    time::TimeTick,
};
use rref::{RRefVec, SharedData};

use crate::{
    domain_helper::{free_domain_resource, FreeShared},
    domain_loader::loader::DomainLoader,
    domain_proxy::ProxyBuilder,
};

#[derive(Debug)]
pub struct EmptyDeviceDomainProxy {
    domain: SRcuData<Box<dyn EmptyDeviceDomain>>,
    lock: Pin<Box<Mutex<()>>>,
    domain_loader: Pin<Box<Mutex<DomainLoader>>>,
    flag: AtomicBool,
    counter: LongLongPerCpu,
    f: AtomicBool,
}

impl EmptyDeviceDomainProxy {
    pub fn new(domain: Box<dyn EmptyDeviceDomain>, domain_loader: DomainLoader) -> Self {
        EmptyDeviceDomainProxy {
            domain: SRcuData::new(domain),
            lock: Box::pin_init(new_mutex!(())).unwrap(),
            domain_loader: Box::pin_init(new_mutex!(domain_loader)).unwrap(),
            flag: AtomicBool::new(false),
            counter: LongLongPerCpu::new(),
            f: AtomicBool::new(false),
        }
    }
}

impl ProxyBuilder for EmptyDeviceDomainProxy {
    type T = Box<dyn EmptyDeviceDomain>;

    fn build(domain: Self::T, domain_loader: DomainLoader) -> Self {
        Self::new(domain, domain_loader)
    }

    fn build_empty(domain_loader: DomainLoader) -> Self {
        Self::new(Box::new(EmptyDeviceDomainEmptyImpl::new()), domain_loader)
    }
    fn build_empty_no_proxy() -> Self::T {
        Box::new(EmptyDeviceDomainEmptyImpl::new())
    }

    fn init_by_box(&self, _argv: Box<dyn Any + Send + Sync>) -> LinuxResult<()> {
        self.init()
    }
}

impl Basic for EmptyDeviceDomainProxy {
    fn domain_id(&self) -> u64 {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._domain_id_with_lock()
        } else {
            self._domain_id_no_lock()
        }
    }
}

impl EmptyDeviceDomain for EmptyDeviceDomainProxy {
    fn init(&self) -> LinuxResult<()> {
        self.domain.read_directly(|domain| domain.init())
    }

    fn read(&self, data: RRefVec<u8>) -> LinuxResult<RRefVec<u8>> {
        // let irq = local_irq_save();
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._read_with_lock(data, 0)
        } else {
            self._read_no_lock(data, 0)
        }
    }

    fn write(&self, data: &RRefVec<u8>) -> LinuxResult<usize> {
        if self.flag.load(core::sync::atomic::Ordering::Relaxed) {
            self._write_with_lock(data)
        } else {
            self._write_no_lock(data)
        }
    }
}

impl EmptyDeviceDomainProxy {
    fn _domain_id(&self) -> u64 {
        self.domain.read_directly(|domain| domain.domain_id())
    }

    fn _domain_id_no_lock(&self) -> u64 {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._domain_id();
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }

    fn _domain_id_with_lock(&self) -> u64 {
        let lock = self.lock.lock();
        let r = self._domain_id();
        drop(lock);
        r
    }

    fn _read(&self, data: RRefVec<u8>) -> LinuxResult<RRefVec<u8>> {
        let (res, old_id) = self.domain.read_directly(|domain| {
            let id = domain.domain_id();
            let old_id = data.move_to(id);
            let r = domain.read(data);
            (r, old_id)
        });
        res.map(|r| {
            r.move_to(old_id);
            r
        })
    }

    fn _write(&self, data: &RRefVec<u8>) -> LinuxResult<usize> {
        self.domain.read_directly(|domain| domain.write(data))
    }

    fn _read_no_lock(&self, data: RRefVec<u8>, irq: u64) -> LinuxResult<RRefVec<u8>> {
        if self.f.load(core::sync::atomic::Ordering::Relaxed) {
            println!("EmptyDeviceDomainProxy _read_no_lock");
        }
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        local_irq_restore(irq);

        let r = self._read(data);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }

    fn _write_no_lock(&self, data: &RRefVec<u8>) -> LinuxResult<usize> {
        self.counter.get_with(|counter| {
            *counter += 1;
        });
        let r = self._write(data);
        self.counter.get_with(|counter| {
            *counter -= 1;
        });
        r
    }

    fn _read_with_lock(&self, data: RRefVec<u8>, irq: u64) -> LinuxResult<RRefVec<u8>> {
        local_irq_restore(irq);
        let lock = self.lock.lock();
        let r = self._read(data);
        drop(lock);
        r
    }

    fn _write_with_lock(&self, data: &RRefVec<u8>) -> LinuxResult<usize> {
        let lock = self.lock.lock();
        let r = self._write(data);
        drop(lock);
        r
    }
}

impl EmptyDeviceDomainProxy {
    pub fn replace(
        &self,
        new_domain: Box<dyn EmptyDeviceDomain>,
        domain_loader: DomainLoader,
    ) -> LinuxResult<()> {
        // println!("EmptyDeviceDomainProxy replace");
        self.f.store(true, core::sync::atomic::Ordering::Relaxed);

        let tick = TimeTick::new("Task Sync");
        let mut loader_guard = self.domain_loader.lock();
        // The writer lock before enable the lock path
        let w_lock = self.lock.lock();
        let old_id = self.domain_id();
        // enable lock path
        self.flag.store(true, core::sync::atomic::Ordering::Relaxed);

        // switch_task_to_cpus();
        sync_cpus();
        // println!("EmptyDeviceDomainProxy replace: switch_task_to_cpus");

        // wait all readers to finish
        while self.counter.sum() != 0 {
            println!("Wait for all reader to finish");
        }
        drop(tick);

        let tick = TimeTick::new("Reinit and state transfer");
        let new_domain_id = new_domain.domain_id();
        new_domain.init().unwrap();
        drop(tick);

        let tick = TimeTick::new("Domain swap");
        // stage4: swap the domain and change to normal state
        let old_domain = self.domain.update_directly(new_domain);

        self.f.store(false, core::sync::atomic::Ordering::Relaxed);
        // disable lock path
        self.flag
            .store(false, core::sync::atomic::Ordering::Relaxed);

        drop(tick);

        let tick = TimeTick::new("Recycle resources");
        // stage5: recycle all resources
        let real_domain = Box::into_inner(old_domain);
        // forget the old domain, it will be dropped by the `free_domain_resource`
        forget(real_domain);

        // We should not free the shared data here, because the shared data will be used
        // in new domain.
        free_domain_resource(old_id, FreeShared::NotFree(new_domain_id));
        drop(tick);

        *loader_guard = domain_loader;
        drop(w_lock);
        drop(loader_guard);
        Ok(())
    }
}

#[derive(Debug)]
pub struct EmptyDeviceDomainEmptyImpl;

impl EmptyDeviceDomainEmptyImpl {
    pub fn new() -> Self {
        EmptyDeviceDomainEmptyImpl
    }
}

impl Basic for EmptyDeviceDomainEmptyImpl {
    fn domain_id(&self) -> u64 {
        u64::MAX
    }
}

impl EmptyDeviceDomain for EmptyDeviceDomainEmptyImpl {
    fn init(&self) -> LinuxResult<()> {
        Ok(())
    }

    fn read(&self, _data: RRefVec<u8>) -> LinuxResult<RRefVec<u8>> {
        Err(LinuxError::ENOSYS)
    }

    fn write(&self, _data: &RRefVec<u8>) -> LinuxResult<usize> {
        Err(LinuxError::ENOSYS)
    }
}
