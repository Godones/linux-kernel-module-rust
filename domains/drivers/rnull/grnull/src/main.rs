#![no_std]
#![no_main]
#![feature(lang_items)]
#![allow(internal_features)]
extern crate alloc;
extern crate malloc;

use alloc::boxed::Box;
use core::panic::PanicInfo;
use basic::domain_main;
use corelib::CoreFunction;
use interface::{null_block::BlockDeviceDomain, Basic};
use rref::{domain_id, SharedHeapAlloc};
use storage::StorageArg;

#[domain_main]
fn main(
    sys: &'static dyn CoreFunction,
    domain_id: u64,
    shared_heap: &'static dyn SharedHeapAlloc,
    storage_arg: StorageArg,
) -> Box<dyn BlockDeviceDomain> {
    // init basic
    corelib::init(sys);
    // init rref's shared heap
    rref::init(shared_heap, domain_id);
    basic::logging::init_logger();
    // init storage
    let StorageArg { allocator, storage } = storage_arg;
    storage::init_database(storage);
    storage::init_data_allocator(allocator);
    // activate the domain
    // interface::activate_domain();
    // call the real blk driver
    rnull::main()
}