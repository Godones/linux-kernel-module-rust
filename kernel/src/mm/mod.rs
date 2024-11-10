use crate::{
    env,
    error::{linux_err, KernelResult},
};

pub mod cache_padded;
pub mod dma;
pub mod folio;
pub mod io_mem;
pub mod mem_cache;
pub mod pages;
pub mod vm;

type SetMemoryX = extern "C" fn(*mut core::ffi::c_void, core::ffi::c_int) -> core::ffi::c_int;

pub fn set_memory_x(virt_addr: usize, numpages: usize) -> KernelResult<()> {
    let raw_set_memory_x = env::SET_MEMORY_X_ADDR as *const u8 as *const ();
    let set_memory_x: SetMemoryX = unsafe { core::mem::transmute(raw_set_memory_x) };
    let ret = set_memory_x(
        virt_addr as *mut core::ffi::c_void,
        numpages as core::ffi::c_int,
    );
    if ret == 0 {
        Ok(())
    } else {
        Err(linux_err::EINVAL)
    }
}
