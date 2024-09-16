use core::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::atomic::AtomicUsize,
};

use memory_addr::PhysAddr;

use crate::config::{FRAME_BITS, FRAME_SIZE};

pub mod ptable;

pub trait PhysPage: Debug + Send + Sync {
    fn phys_addr(&self) -> PhysAddr;
    fn as_bytes(&self) -> &[u8];
    fn as_mut_bytes(&mut self) -> &mut [u8];
    fn read_value_atomic(&self, offset: usize) -> usize;
    fn write_value_atomic(&mut self, offset: usize, value: usize);
}

#[derive(Debug)]
pub struct FrameTracker {
    start_page: usize,
    page_count: usize,
    dealloc: bool,
}

impl FrameTracker {
    pub fn new(start_page: usize, page_count: usize, dealloc: bool) -> Self {
        Self {
            start_page,
            page_count,
            dealloc,
        }
    }
    pub fn start(&self) -> usize {
        self.start_page << FRAME_BITS
    }
}

impl PhysPage for FrameTracker {
    fn phys_addr(&self) -> PhysAddr {
        PhysAddr::from(self.start())
    }

    fn as_bytes(&self) -> &[u8] {
        self.deref()
    }

    fn as_mut_bytes(&mut self) -> &mut [u8] {
        self.deref_mut()
    }
    fn read_value_atomic(&self, offset: usize) -> usize {
        let ptr = self.start() + offset;
        unsafe {
            AtomicUsize::from_ptr(ptr as *mut usize).load(core::sync::atomic::Ordering::Relaxed)
        }
    }
    fn write_value_atomic(&mut self, offset: usize, value: usize) {
        let ptr = self.start() + offset;
        unsafe {
            AtomicUsize::from_ptr(ptr as *mut usize)
                .store(value, core::sync::atomic::Ordering::Relaxed)
        }
    }
}

impl Drop for FrameTracker {
    fn drop(&mut self) {
        unimplemented!("drop FrameTracker")
    }
}

impl Deref for FrameTracker {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        unsafe {
            core::slice::from_raw_parts(self.start() as *const u8, FRAME_SIZE * self.page_count)
        }
    }
}
impl DerefMut for FrameTracker {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe {
            core::slice::from_raw_parts_mut(self.start() as *mut u8, FRAME_SIZE * self.page_count)
        }
    }
}

pub fn free_frames(addr: *mut u8, num: usize) {
    assert_eq!(num.next_power_of_two(), num);
    unimplemented!("free_frames")
}

pub fn alloc_frames(num: usize) -> *mut u8 {
    assert_eq!(num.next_power_of_two(), num);
    unimplemented!("alloc_frames")
}
pub fn alloc_frame_trackers(count: usize) -> FrameTracker {
    unimplemented!("alloc_frame_trackers")
}

/// Allocate a free region in kernel space.
pub fn alloc_free_region(size: usize) -> Option<usize> {
    assert!(size > 0 && size % FRAME_SIZE == 0);
    unimplemented!("alloc_free_region")
}
pub fn query_kernel_space(addr: usize) -> Option<usize> {
    unimplemented!("query_kernel_space")
}

pub fn unmap_region_from_kernel(addr: usize) -> Result<(), &'static str> {
    assert_eq!(addr % FRAME_SIZE, 0);
    unimplemented!("unmap_region_from_kernel")
}

#[no_mangle]
static sbss: usize = 0;
#[no_mangle]
static ebss: usize = 0;
