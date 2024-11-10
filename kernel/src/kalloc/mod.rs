mod allocator;

pub use allocator::{BoxExt, UniqueArc};

#[global_allocator]
static ALLOCATOR: allocator::KernelAllocator = allocator::KernelAllocator;
