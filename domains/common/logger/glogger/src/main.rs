#![no_std]
#![no_main]
#![feature(lang_items)]
#![allow(internal_features)]
extern crate alloc;
extern crate malloc;
use alloc::boxed::Box;
use core::panic::PanicInfo;
use corelib::CoreFunction;
use interface::logger::LogDomain;
use rref::{domain_id, SharedHeapAlloc};
use storage::StorageArg;


#[no_mangle]
fn main(
    sys: &'static dyn CoreFunction,
    domain_id: u64,
    shared_heap: &'static dyn SharedHeapAlloc,
    storage_arg: StorageArg,
) -> Box<dyn LogDomain> {
    // init basic
    corelib::init(sys);
    // init rref's shared heap
    rref::init(shared_heap, domain_id);
    // basic::logging::init_logger();
    // init storage
    let StorageArg { allocator, storage } = storage_arg;
    storage::init_database(storage);
    storage::init_data_allocator(allocator);
    // call the real blk driver
    logger::main()
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(p) = info.location() {
        basic::println_color!(
                    31,
                    "line {}, file {}: {}",
                    p.line(),
                    p.file(),
                    info.message()
                );
    } else {
        basic::println_color!(31, "no location information available");
    }
    basic::backtrace(domain_id());
    loop {

    }
}