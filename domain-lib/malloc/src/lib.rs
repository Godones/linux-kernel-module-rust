#![no_std]
use core::alloc::GlobalAlloc;

use buddy_system_allocator::LockedHeap;
use rref::domain_id;

#[global_allocator]
static HEAP_ALLOCATOR: HeapAllocator = HeapAllocator::new();

pub struct HeapAllocator {
    allocator: LockedHeap<32>,
}

impl HeapAllocator {
    pub const fn new() -> Self {
        Self {
            allocator: LockedHeap::<32>::new(),
        }
    }
}

impl Default for HeapAllocator {
    fn default() -> Self {
        Self::new()
    }
}

unsafe impl GlobalAlloc for HeapAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let ptr = self.allocator.alloc(layout);
        if ptr.is_null() {
            let need_pages = (layout.size() + 4096 - 1) / 4096;
            let need_pages = (need_pages * 2).next_power_of_two();
            // we alloc two times of the pages we need
            let new_pages = corelib::alloc_raw_pages(need_pages, domain_id());
            assert!(!new_pages.is_null());
            self.allocator
                .lock()
                .add_to_heap(new_pages as usize, need_pages * 4096 + new_pages as usize);
            let ptr = self.allocator.alloc(layout);
            assert!(
                !ptr.is_null(),
                "need pages: {}, layout:{:#x?}",
                need_pages,
                layout
            );
            ptr
        } else {
            ptr
        }
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: core::alloc::Layout) {
        self.allocator.dealloc(ptr, layout);
    }
}
