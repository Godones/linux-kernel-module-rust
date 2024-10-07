use core::convert::TryInto;

use crate::{bindings, error};

/// Fills `dest` with random bytes generated from the kernel's CSPRNG. Ensures
/// that the CSPRNG has been seeded before generating any random bytes, and
/// will block until it's ready.
pub fn getrandom(dest: &mut [u8]) -> error::KernelResult<()> {
    let res = unsafe { bindings::wait_for_random_bytes() };
    if res != 0 {
        return Err(error::Error::from_errno(res));
    }

    unsafe {
        bindings::get_random_bytes(
            dest.as_mut_ptr() as *mut core::ffi::c_void,
            dest.len().try_into().unwrap(),
        );
    }
    Ok(())
}

/// Fills `dest` with random bytes generated from the kernel's CSPRNG. If the
/// CSPRNG is not yet seeded, returns an `Err(EAGAIN)` immediately. Only
/// available on 4.19 and later kernels.
pub fn getrandom_nonblock(dest: &mut [u8]) -> error::KernelResult<()> {
    if !unsafe { bindings::rng_is_initialized() } {
        return Err(error::linux_err::EAGAIN);
    }
    getrandom(dest)
}

/// Contributes the contents of `data` to the kernel's entropy pool. Does _not_
/// credit the kernel entropy counter though.
pub fn add_randomness(data: &[u8]) {
    unsafe {
        bindings::add_device_randomness(
            data.as_ptr() as *const core::ffi::c_void,
            data.len().try_into().unwrap(),
        );
    }
}
