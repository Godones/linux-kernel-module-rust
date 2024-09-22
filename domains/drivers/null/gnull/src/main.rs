#![no_std]
#![no_main]
#![feature(lang_items)]
#![allow(internal_features)]

use core::panic::PanicInfo;

#[no_mangle]
fn main()->usize {
    add(1, 2) as usize
}



fn add(a: i32, b: i32) -> i32 {
    a + b
}


#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {

    }
}