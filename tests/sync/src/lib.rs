#![no_std]
extern crate alloc;
use alloc::{boxed::Box, format};
use core::{
    fmt::Debug,
    pin::Pin,
    sync::atomic::{AtomicUsize, Ordering},
};

use kernel::{
    init::InPlaceInit,
    module, new_mutex, new_spinlock, println,
    sync::{Mutex, RcuData, SRcuData, SpinLock},
    Module, ThisModule,
};
use spin::Lazy;

struct SyncModule;

static GLOBAL: Lazy<Pin<Box<SpinLock<usize>>>> =
    Lazy::new(|| Box::pin_init(new_spinlock!(0)).unwrap());

fn global_synchronization_example() {
    let mut global = GLOBAL.lock();
    *global = 1;
}

fn return_dyn_data() -> Box<dyn Debug> {
    static C: AtomicUsize = AtomicUsize::new(0);
    let c = C.fetch_add(1, Ordering::SeqCst);
    Box::new(format!("Hello from return_dyn_data {}", c))
}

fn rcu_example() {
    let data = RcuData::new(10);
    data.read(|v| {
        println!("RcuData is {}", v);
    });

    data.update(20);
    data.read(|v| {
        println!("New RcuData is {}", v);
    });

    let data = RcuData::new(return_dyn_data());
    data.read(|v| {
        println!("RcuData is {:?}", v);
    });

    data.update(return_dyn_data());
    data.read(|v| {
        println!("New RcuData is {:?}", v);
    });

    println!("rcu_example done");
}

fn srcu_example() {
    let mut data = SRcuData::new(10);
    data.read(|v| {
        println!("SRcuData is {}", v);
    });

    data.update(20);
    data.read(|v| {
        println!("New SRcuData is {}", v);
    });

    let data = RcuData::new(return_dyn_data());
    data.read(|v| {
        println!("SRcuData is {:?}", v);
    });

    data.update(return_dyn_data());
    data.read(|v| {
        println!("New SRcuData is {:?}", v);
    });

    println!("srcu_example done");
}

fn lock_example() {
    global_synchronization_example();
    let spinlock_data = Box::pin_init(new_spinlock!(10)).unwrap();
    println!("Data {} is locked by a spinlock", *spinlock_data.lock());
    let mutex_data = Box::pin_init(new_mutex!(50)).unwrap();
    let mut data = mutex_data.lock();
    println!("Data {} is locked by a mutex", *data);
    *data = 100;
    println!("Now data is {}", *data);
}

impl Module for SyncModule {
    fn init(_module: &'static ThisModule) -> kernel::error::KernelResult<Self> {
        println!("Test kernel sync primitives");
        lock_example();
        rcu_example();
        srcu_example();
        Ok(SyncModule)
    }
}

impl Drop for SyncModule {
    fn drop(&mut self) {
        println!("Goodbye kernel module!");
    }
}

module! {
    type: SyncModule,
    name: "SyncModule",
    author: "godones",
    description: "Test kernel sync primitives",
    license: "GPL",
}
