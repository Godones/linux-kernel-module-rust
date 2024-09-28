#![no_std]
extern crate alloc;
use alloc::{boxed::Box, format};
use core::{
    fmt::Debug,
    sync::atomic::{AtomicUsize, Ordering},
};

use kbind::{
    println,
    sync::{Mutex, RcuData, Spinlock},
};
use spin::Lazy;

struct SyncModule;

static GLOBAL: Lazy<Spinlock<i32>> = Lazy::new(|| Spinlock::new(0));

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
    let mut data = RcuData::new(10);
    data.read(|v| {
        println!("RcuData is {}", v);
    });

    data.update(20);
    data.read(|v| {
        println!("New RcuData is {}", v);
    });

    let mut data = RcuData::new(return_dyn_data());
    data.read(|v| {
        println!("RcuData is {:?}", v);
    });

    data.update(return_dyn_data());
    data.read(|v| {
        println!("New RcuData is {:?}", v);
    });

    println!("rcu_example done");
}

fn lock_example() {
    global_synchronization_example();
    let spinlock_data = Spinlock::new(100);
    println!("Data {} is locked by a spinlock", *spinlock_data.lock());
    let mutex_data = Mutex::new(50);
    let mut data = mutex_data.lock();
    println!("Data {} is locked by a mutex", *data);
    *data = 100;
    println!("Now data is {}", *data);
}

impl kbind::KernelModule for SyncModule {
    fn init() -> kbind::KernelResult<Self> {
        println!("Test kernel sync primitives");
        lock_example();
        rcu_example();
        Ok(SyncModule)
    }
}

impl Drop for SyncModule {
    fn drop(&mut self) {
        println!("Goodbye kernel module!");
    }
}

kbind::kernel_module!(
    SyncModule,
    author: b"godones",
    description: b"Test kernel sync primitives",
    license: b"GPL"
);
