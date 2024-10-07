use alloc::{vec, vec::Vec};

use crate::error;

pub struct KernelSlicePtr(*mut core::ffi::c_void, usize);

impl KernelSlicePtr {
    /// Construct a user slice from a raw pointer and a length in bytes.
    ///
    /// Checks that the provided range is within the legal area for
    /// userspace memory, using `access_ok` (e.g., on i386, the range
    /// must be within the first 3 gigabytes), but does not check that
    /// the actual pages are mapped in the current process with
    /// appropriate permissions. Those checks are handled in the read
    /// and write methods.
    ///
    /// This is `unsafe` because if it is called within `set_fs(KERNEL_DS)` context then
    /// `access_ok` will not do anything. As a result the only place you can safely use this is
    /// with an `__user` pointer that was provided by the kernel.
    pub(crate) unsafe fn new(
        ptr: *mut core::ffi::c_void,
        length: usize,
    ) -> error::KernelResult<KernelSlicePtr> {
        Ok(KernelSlicePtr(ptr, length))
    }

    /// Read the entirety of the user slice and return it in a `Vec`.
    ///
    /// Returns EFAULT if the address does not currently point to
    /// mapped, readable memory.
    pub fn read_all(self) -> error::KernelResult<Vec<u8>> {
        self.reader().read_all()
    }

    /// Construct a `UserSlicePtrReader` that can incrementally read
    /// from the user slice.
    pub fn reader(self) -> KernelSlicePtrReader {
        KernelSlicePtrReader(self.0, self.1)
    }

    /// Write the provided slice into the user slice.
    ///
    /// Returns EFAULT if the address does not currently point to
    /// mapped, writable memory (in which case some data from before the
    /// fault may be written), or `data` is larger than the user slice
    /// (in which case no data is written).
    pub fn write_all(self, data: &[u8]) -> error::KernelResult<()> {
        self.writer().write(data)
    }

    /// Construct a `UserSlicePtrWrite` that can incrementally write
    /// into the user slice.
    pub fn writer(self) -> KernelSlicePtrWriter {
        KernelSlicePtrWriter(self.0, self.1)
    }
}

pub struct KernelSlicePtrReader(*mut core::ffi::c_void, usize);

impl KernelSlicePtrReader {
    /// Returns the number of bytes left to be read from this. Note that even
    /// reading less than this number of bytes may return an Error().
    pub fn len(&self) -> usize {
        self.1
    }

    /// Returns `true` if `self.len()` is 0.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Read all data remaining in the user slice and return it in a `Vec`.
    ///
    /// Returns EFAULT if the address does not currently point to
    /// mapped, readable memory.
    pub fn read_all(&mut self) -> error::KernelResult<Vec<u8>> {
        let mut data = vec![0; self.1];
        self.read(&mut data)?;
        Ok(data)
    }

    pub fn read(&mut self, data: &mut [u8]) -> error::KernelResult<()> {
        if data.len() > self.1 || data.len() > u32::MAX as usize {
            return Err(error::linux_err::EFAULT);
        }
        unsafe {
            core::ptr::copy_nonoverlapping(self.0 as *const u8, data.as_mut_ptr(), data.len());
        }
        // Since this is not a pointer to a valid object in our program,
        // we cannot use `add`, which has C-style rules for defined
        // behavior.
        self.0 = self.0.wrapping_add(data.len());
        self.1 -= data.len();
        Ok(())
    }
}

pub struct KernelSlicePtrWriter(*mut core::ffi::c_void, usize);

impl KernelSlicePtrWriter {
    pub fn len(&self) -> usize {
        self.1
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn write(&mut self, data: &[u8]) -> error::KernelResult<()> {
        if data.len() > self.1 || data.len() > u32::MAX as usize {
            return Err(error::linux_err::EFAULT);
        }
        unsafe {
            core::ptr::copy_nonoverlapping(data.as_ptr(), self.0 as *mut u8, data.len());
        }
        // Since this is not a pointer to a valid object in our program,
        // we cannot use `add`, which has C-style rules for defined
        // behavior.
        self.0 = self.0.wrapping_add(data.len());
        self.1 -= data.len();
        Ok(())
    }
}
