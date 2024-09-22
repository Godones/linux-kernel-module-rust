use linux_kernel_module::{
    kalloc::vm::{alloc_module_area, ModuleArea},
    KernelResult,
};
pub const PAGE_SIZE: usize = 4096;

pub fn alloc_free_region(size: usize) -> KernelResult<ModuleArea> {
    assert_eq!(size % PAGE_SIZE, 0);
    alloc_module_area(size)
}

bitflags::bitflags! {
    /// Generic page table entry flags that indicate the corresponding mapped
    /// memory region permissions and attributes.
    #[derive(Debug,Copy, Clone)]
    pub struct MappingFlags: usize {
        /// The memory is readable.
        const READ          = 1 << 0;
        /// The memory is writable.
        const WRITE         = 1 << 1;
        /// The memory is executable.
        const EXECUTE       = 1 << 2;
    }
}
