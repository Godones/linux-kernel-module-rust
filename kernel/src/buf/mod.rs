mod kernel_ptr;
mod user_ptr;

use alloc::vec::Vec;
use core::ffi::{c_ulong, c_void};
use core::mem::MaybeUninit;
pub use kernel_ptr::*;
pub use user_ptr::*;
use crate::bindings;
use crate::code::EFAULT;
use crate::error::KernelResult as Result;
use crate::kalloc::AllocFlags;
use crate::types::FromBytes;

/// The type used for userspace addresses.
pub type UserPtr = usize;
/// A reader for [`UserSlice`].
///
/// Used to incrementally read from the user slice.
pub struct UserSliceReader {
    ptr: UserPtr,
    length: usize,
}


impl UserSliceReader {
    /// Skip the provided number of bytes.
    ///
    /// Returns an error if skipping more than the length of the buffer.
    pub fn skip(&mut self, num_skip: usize) -> Result {
        // Update `self.length` first since that's the fallible part of this operation.
        self.length = self.length.checked_sub(num_skip).ok_or(EFAULT)?;
        self.ptr = self.ptr.wrapping_add(num_skip);
        Ok(())
    }

    /// Create a reader that can access the same range of data.
    ///
    /// Reading from the clone does not advance the current reader.
    ///
    /// The caller should take care to not introduce TOCTOU issues, as described in the
    /// documentation for [`UserSlice`].
    pub fn clone_reader(&self) -> UserSliceReader {
        UserSliceReader {
            ptr: self.ptr,
            length: self.length,
        }
    }

    /// Returns the number of bytes left to be read from this reader.
    ///
    /// Note that even reading less than this number of bytes may fail.
    pub fn len(&self) -> usize {
        self.length
    }

    /// Returns `true` if no data is available in the io buffer.
    pub fn is_empty(&self) -> bool {
        self.length == 0
    }

    /// Reads raw data from the user slice into a kernel buffer.
    ///
    /// For a version that uses `&mut [u8]`, please see [`UserSliceReader::read_slice`].
    ///
    /// Fails with [`EFAULT`] if the read happens on a bad address, or if the read goes out of
    /// bounds of this [`UserSliceReader`]. This call may modify `out` even if it returns an error.
    ///
    /// # Guarantees
    ///
    /// After a successful call to this method, all bytes in `out` are initialized.
    pub fn read_raw(&mut self, out: &mut [MaybeUninit<u8>]) -> Result {
        let len = out.len();
        let out_ptr = out.as_mut_ptr().cast::<c_void>();
        if len > self.length {
            return Err(EFAULT);
        }
        let Ok(len_ulong) = c_ulong::try_from(len) else {
            return Err(EFAULT);
        };
        // SAFETY: `out_ptr` points into a mutable slice of length `len_ulong`, so we may write
        // that many bytes to it.
        let res =
            unsafe { bindings::_copy_from_user(out_ptr, self.ptr as *const c_void, len_ulong) };
        if res != 0 {
            return Err(EFAULT);
        }
        self.ptr = self.ptr.wrapping_add(len);
        self.length -= len;
        Ok(())
    }

    /// Reads raw data from the user slice into a kernel buffer.
    ///
    /// Fails with [`EFAULT`] if the read happens on a bad address, or if the read goes out of
    /// bounds of this [`UserSliceReader`]. This call may modify `out` even if it returns an error.
    pub fn read_slice(&mut self, out: &mut [u8]) -> Result {
        // SAFETY: The types are compatible and `read_raw` doesn't write uninitialized bytes to
        // `out`.
        let out = unsafe { &mut *(out as *mut [u8] as *mut [MaybeUninit<u8>]) };
        self.read_raw(out)
    }

    /// Reads a value of the specified type.
    ///
    /// Fails with [`EFAULT`] if the read happens on a bad address, or if the read goes out of
    /// bounds of this [`UserSliceReader`].
    pub fn read<T: FromBytes>(&mut self) -> Result<T> {
        let len = size_of::<T>();
        if len > self.length {
            return Err(EFAULT);
        }
        let Ok(len_ulong) = c_ulong::try_from(len) else {
            return Err(EFAULT);
        };
        let mut out: MaybeUninit<T> = MaybeUninit::uninit();
        // SAFETY: The local variable `out` is valid for writing `size_of::<T>()` bytes.
        //
        // By using the _copy_from_user variant, we skip the check_object_size check that verifies
        // the kernel pointer. This mirrors the logic on the C side that skips the check when the
        // length is a compile-time constant.
        let res = unsafe {
            bindings::_copy_from_user(
                out.as_mut_ptr().cast::<c_void>(),
                self.ptr as *const c_void,
                len_ulong,
            )
        };
        if res != 0 {
            return Err(EFAULT);
        }
        self.ptr = self.ptr.wrapping_add(len);
        self.length -= len;
        // SAFETY: The read above has initialized all bytes in `out`, and since `T` implements
        // `FromBytes`, any bit-pattern is a valid value for this type.
        Ok(unsafe { out.assume_init() })
    }

    /// Reads the entirety of the user slice, appending it to the end of the provided buffer.
    ///
    /// Fails with [`EFAULT`] if the read happens on a bad address.
    pub fn read_all(mut self, buf: &mut Vec<u8>, _flags: AllocFlags) -> Result {
        let len = self.length;
        Vec::<u8>::try_reserve(buf, len)?;

        // The call to `try_reserve` was successful, so the spare capacity is at least `len` bytes
        // long.
        self.read_raw(&mut buf.spare_capacity_mut()[..len])?;

        // SAFETY: Since the call to `read_raw` was successful, so the next `len` bytes of the
        // vector have been initialized.
        unsafe { buf.set_len(buf.len() + len) };
        Ok(())
    }
}