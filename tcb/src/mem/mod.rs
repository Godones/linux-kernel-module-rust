use kbind::{mm, mm::vm::VSpace};

use crate::config::FRAME_SIZE;

pub fn free_frames(addr: *mut u8, num: usize) {
    assert_eq!(num.next_power_of_two(), num);
    let vspace = unsafe { VSpace::from_raw(addr as usize, num * FRAME_SIZE) };
    drop(vspace);
}

pub fn alloc_frames(num: usize) -> *mut u8 {
    assert_eq!(num.next_power_of_two(), num);
    let vspace = mm::vm::alloc_contiguous_vspace(num * FRAME_SIZE).unwrap();
    let ptr = vspace.as_ptr();
    core::mem::forget(vspace);
    ptr
}

#[no_mangle]
static sbss: usize = 0;
#[no_mangle]
static ebss: usize = 0;
