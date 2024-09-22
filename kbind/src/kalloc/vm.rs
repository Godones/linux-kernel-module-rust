use crate::{bindings, code, env, pr_warning, KernelResult};

type ModuleAllocFn = extern "C" fn(size: core::ffi::c_ulong) -> *mut core::ffi::c_void;
type ModuleMemFreeFn = extern "C" fn(*mut core::ffi::c_void);
/// Allocates a memory region of the given size in the kernel's virtual address space.
fn module_alloc(size: usize) -> usize {
    let raw_module_alloc = env::MODULE_ALLOC_ADDR as *const u8 as *const ();
    let module_alloc: ModuleAllocFn = unsafe { core::mem::transmute(raw_module_alloc) };
    let virt_ptr = module_alloc(size as _) as usize;
    virt_ptr
}

/// Frees a memory region previously allocated with [module_alloc]
fn module_free(virt_ptr: usize) {
    let raw_module_memfree = env::MODULE_MEMFREE_ADDR as *const u8 as *const ();
    let module_memfree: ModuleMemFreeFn = unsafe { core::mem::transmute(raw_module_memfree) };
    module_memfree(virt_ptr as *mut core::ffi::c_void)
}

#[derive(Debug)]
pub struct ModuleArea {
    start: usize,
    size: usize,
}

impl ModuleArea {
    pub fn as_ptr(&self) -> *mut u8 {
        self.start as *mut u8
    }
    pub fn as_slice(&self) -> &[u8] {
        unsafe { core::slice::from_raw_parts(self.start as *const u8, self.size) }
    }
    pub fn as_mut_slice(&self) -> &mut [u8] {
        unsafe { core::slice::from_raw_parts_mut(self.start as *mut u8, self.size) }
    }
    pub fn len(&self) -> usize {
        self.size
    }
}

impl Drop for ModuleArea {
    fn drop(&mut self) {
        module_free(self.start);
        pr_warning!("Dropping VirtArea: {:x?}", self.start);
    }
}

/// Allocates a virtual memory area of the given size. The size must be a multiple of the page size.
pub fn alloc_module_area(size: usize) -> KernelResult<ModuleArea> {
    assert_eq!(size % 4096, 0);
    let start = module_alloc(size);
    if start == 0 {
        Err(code::ENOMEM)
    } else {
        Ok(ModuleArea { start, size })
    }
}

#[derive(Debug)]
pub struct VSpace {
    start: usize,
    size: usize,
}

impl VSpace {
    fn new(start: usize, size: usize) -> Self {
        VSpace { start, size }
    }

    pub fn len(&self) -> usize {
        self.size
    }
}

impl Drop for VSpace {
    fn drop(&mut self) {
        unsafe {
            bindings::vfree(self.start as *mut core::ffi::c_void);
        }
        pr_warning!("Dropping VSpace: {:x?}", self.start);
    }
}

pub fn alloc_contiguous_vspace(size: usize) -> KernelResult<VSpace> {
    let start = unsafe { bindings::vzalloc(size as _) } as usize;
    assert_eq!(size % 4096, 0);
    assert_eq!(start % 4096, 0);
    if start == 0 {
        Err(code::ENOMEM)
    } else {
        Ok(VSpace::new(start, size))
    }
}
