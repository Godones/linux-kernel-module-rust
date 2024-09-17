use core::{fmt::Debug, num::TryFromIntError};

use crate::bindings;

pub struct Error(core::ffi::c_int);

impl Error {
    pub const EINVAL: Self = Error(-(bindings::EINVAL as i32));
    pub const ENOMEM: Self = Error(-(bindings::ENOMEM as i32));
    pub const EFAULT: Self = Error(-(bindings::EFAULT as i32));
    pub const ESPIPE: Self = Error(-(bindings::ESPIPE as i32));
    pub const EAGAIN: Self = Error(-(bindings::EAGAIN as i32));

    pub fn from_kernel_errno(errno: core::ffi::c_int) -> Error {
        Error(errno)
    }

    pub fn to_kernel_errno(&self) -> core::ffi::c_int {
        self.0
    }
}

impl From<TryFromIntError> for Error {
    fn from(_: TryFromIntError) -> Error {
        Error::EINVAL
    }
}

pub type KernelResult<T> = Result<T, Error>;

impl Debug for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(match (-self.0) as u32 {
            bindings::EINVAL => "EINVAL",
            bindings::ENOMEM => "ENOMEM",
            bindings::EFAULT => "EFAULT",
            bindings::ESPIPE => "ESPIPE",
            bindings::EAGAIN => "EAGAIN",
            _ => "Unknown error",
        })
    }
}
