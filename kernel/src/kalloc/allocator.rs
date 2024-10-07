use core::{
    alloc::{GlobalAlloc, Layout},
    ffi::c_ulong,
    ptr,
};

use crate::bindings;

pub struct KernelAllocator;

unsafe impl GlobalAlloc for KernelAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // krealloc is used instead of kmalloc because kmalloc is an inline function and can't be
        // bound to as a result
        if layout.size() < 4096 {
            bindings::krealloc(ptr::null(), layout.size(), bindings::GFP_KERNEL) as *mut u8
        } else {
            bindings::vzalloc(layout.size() as c_ulong) as *mut u8
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if layout.size() < 4096 {
            bindings::kfree(ptr as *mut core::ffi::c_void);
        } else {
            bindings::vfree(ptr as *mut core::ffi::c_void);
        }
    }
}
