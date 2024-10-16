use alloc::collections::TryReserveError;
use core::{
    alloc::{AllocError, LayoutError},
    ffi::CStr,
    fmt,
    fmt::Debug,
    num::TryFromIntError,
    str::Utf8Error,
};

use log::warn;

use crate::bindings;

pub type KernelResult<T = (), E = Error> = Result<T, E>;

pub struct Error(core::ffi::c_int);

impl Error {
    pub fn from_errno(errno: core::ffi::c_int) -> Error {
        if errno < -(bindings::MAX_ERRNO as i32) || errno >= 0 {
            // TODO: Make it a `WARN_ONCE` once available.
            warn!(
                "attempted to create `Error` with out of range `errno`: {}",
                errno
            );
            return linux_err::EINVAL;
        }
        // INVARIANT: The check above ensures the type invariant
        // will hold.
        Error(errno)
    }
    /// Creates an [`Error`] from a kernel error code.
    ///
    /// # Safety
    ///
    /// `errno` must be within error code range (i.e. `>= -MAX_ERRNO && < 0`).
    #[allow(unused)]
    unsafe fn from_errno_unchecked(errno: core::ffi::c_int) -> Error {
        // INVARIANT: The contract ensures the type invariant
        // will hold.
        Error(errno)
    }

    pub fn to_errno(&self) -> core::ffi::c_int {
        self.0
    }

    /// Returns the error encoded as a pointer.
    pub(crate) fn to_ptr<T>(self) -> *mut T {
        // SAFETY: `self.0` is a valid error due to its invariant.
        crate::sys_err_ptr(self.0.into()) as *mut _
    }

    /// Returns a string representing the error, if one exists.
    pub fn name(&self) -> Option<&'static CStr> {
        // SAFETY: Just an FFI call, there are no extra safety requirements.
        let ptr = crate::sys_errname(-self.0);
        if ptr.is_null() {
            None
        } else {
            // SAFETY: The string returned by `errname` is static and `NUL`-terminated.
            Some(unsafe { CStr::from_ptr(ptr) })
        }
    }

    pub fn to_blk_status(self) -> bindings::blk_status_t {
        crate::sys_errno_to_blk_status(self.0)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.name() {
            // Print out number if no name can be found.
            None => f.debug_tuple("Error").field(&-self.0).finish(),
            // SAFETY: These strings are ASCII-only.
            Some(name) => f
                .debug_tuple(unsafe { core::str::from_utf8_unchecked(name.to_bytes()) })
                .finish(),
        }
    }
}

/// Contains the C-compatible error codes.
#[rustfmt::skip]
#[allow(unused)]
pub mod linux_err {
    macro_rules! declare_err {
        ($err:tt $(,)? $($doc:expr),+) => {
            $(
            #[doc = $doc]
            )*
            pub const $err: super::Error = super::Error(-(crate::bindings::$err as i32));
        };
    }

    declare_err!(EPERM, "Operation not permitted.");
    declare_err!(ENOENT, "No such file or directory.");
    declare_err!(ESRCH, "No such process.");
    declare_err!(EINTR, "Interrupted system call.");
    declare_err!(EIO, "I/O error.");
    declare_err!(ENXIO, "No such device or address.");
    declare_err!(E2BIG, "Argument list too long.");
    declare_err!(ENOEXEC, "Exec format error.");
    declare_err!(EBADF, "Bad file number.");
    declare_err!(ECHILD, "No child processes.");
    declare_err!(EAGAIN, "Try again.");
    declare_err!(ENOMEM, "Out of memory.");
    declare_err!(EACCES, "Permission denied.");
    declare_err!(EFAULT, "Bad address.");
    declare_err!(ENOTBLK, "Block device required.");
    declare_err!(EBUSY, "Device or resource busy.");
    declare_err!(EEXIST, "File exists.");
    declare_err!(EXDEV, "Cross-device link.");
    declare_err!(ENODEV, "No such device.");
    declare_err!(ENOTDIR, "Not a directory.");
    declare_err!(EISDIR, "Is a directory.");
    declare_err!(EINVAL, "Invalid argument.");
    declare_err!(ENFILE, "File table overflow.");
    declare_err!(EMFILE, "Too many open files.");
    declare_err!(ENOTTY, "Not a typewriter.");
    declare_err!(ETXTBSY, "Text file busy.");
    declare_err!(EFBIG, "File too large.");
    declare_err!(ENOSPC, "No space left on device.");
    declare_err!(ESPIPE, "Illegal seek.");
    declare_err!(EROFS, "Read-only file system.");
    declare_err!(EMLINK, "Too many links.");
    declare_err!(EPIPE, "Broken pipe.");
    declare_err!(EDOM, "Math argument out of domain of func.");
    declare_err!(ERANGE, "Math result not representable.");
    declare_err!(ERESTARTSYS, "Restart the system call.");
    declare_err!(ERESTARTNOINTR, "System call was interrupted by a signal and will be restarted.");
    declare_err!(ERESTARTNOHAND, "Restart if no handler.");
    declare_err!(ENOIOCTLCMD, "No ioctl command.");
    declare_err!(ERESTART_RESTARTBLOCK, "Restart by calling sys_restart_syscall.");
    declare_err!(EPROBE_DEFER, "Driver requests probe retry.");
    declare_err!(EOPENSTALE, "Open found a stale dentry.");
    declare_err!(ENOPARAM, "Parameter not supported.");
    declare_err!(EBADHANDLE, "Illegal NFS file handle.");
    declare_err!(ENOTSYNC, "Update synchronization mismatch.");
    declare_err!(EBADCOOKIE, "Cookie is stale.");
    declare_err!(ENOTSUPP, "Operation is not supported.");
    declare_err!(ETOOSMALL, "Buffer or request is too small.");
    declare_err!(ESERVERFAULT, "An untranslatable error occurred.");
    declare_err!(EBADTYPE, "Type not supported by server.");
    declare_err!(EJUKEBOX, "Request initiated, but will not complete before timeout.");
    declare_err!(EIOCBQUEUED, "iocb queued, will get completion event.");
    declare_err!(ERECALLCONFLICT, "Conflict with recalled state.");
    declare_err!(ENOGRACE, "NFS file lock reclaim refused.");
    declare_err!(ENODATA, "No data available.");
    declare_err!(EOPNOTSUPP, "Operation not supported on transport endpoint.");
    declare_err!(ENOSYS, "Invalid system call number.");
    declare_err!(ESTALE, "Stale file handle.");
    declare_err!(EUCLEAN, "Structure needs cleaning.");
}

impl From<AllocError> for Error {
    fn from(_: AllocError) -> Error {
        linux_err::ENOMEM
    }
}

impl From<TryFromIntError> for Error {
    fn from(_: TryFromIntError) -> Error {
        linux_err::EINVAL
    }
}

impl From<Utf8Error> for Error {
    fn from(_: Utf8Error) -> Error {
        linux_err::EINVAL
    }
}

impl From<fmt::Error> for Error {
    fn from(_: fmt::Error) -> Error {
        linux_err::EINVAL
    }
}

impl From<core::convert::Infallible> for Error {
    fn from(e: core::convert::Infallible) -> Error {
        match e {
            // SAFETY: `Infallible` is uninhabited.
        }
    }
}

impl From<TryReserveError> for Error {
    fn from(_: TryReserveError) -> Error {
        linux_err::ENOMEM
    }
}

impl From<LayoutError> for Error {
    fn from(_: LayoutError) -> Error {
        linux_err::ENOMEM
    }
}

/// Converts an integer as returned by a C kernel function to an error if it's negative, and
/// `Ok(())` otherwise.
pub fn to_result(err: core::ffi::c_int) -> KernelResult<()> {
    if err < 0 {
        Err(Error::from_errno(err))
    } else {
        Ok(())
    }
}

/// Calls a closure returning a [`crate::error::Result<T>`] and converts the result to
/// a C integer result.
///
/// This is useful when calling Rust functions that return [`crate::error::Result<T>`]
/// from inside `extern "C"` functions that need to return an integer error result.
///
/// `T` should be convertible from an `i16` via `From<i16>`.
///
/// # Examples
///
/// ```ignore
/// # use kernel::from_result;
/// # use kernel::bindings;
/// unsafe extern "C" fn probe_callback(
///     pdev: *mut bindings::platform_device,
/// ) -> core::ffi::c_int {
///     from_result(|| {
///         let ptr = devm_alloc(pdev)?;
///         bindings::platform_set_drvdata(pdev, ptr);
///         Ok(0)
///     })
/// }
/// ```
// TODO: Remove `dead_code` marker once an in-kernel client is available.
#[allow(dead_code)]
pub fn from_result<T, F>(f: F) -> T
where
    T: From<i16>,
    F: FnOnce() -> KernelResult<T>,
{
    f().unwrap_or_else(|e| T::from(e.to_errno() as i16))
}

/// Error message for calling a default function of a [`#[vtable]`](macros::vtable) trait.
pub const VTABLE_DEFAULT_ERROR: &str =
    "This function must not be called, see the #[vtable] documentation.";

/// Transform a kernel "error pointer" to a normal pointer.
///
/// Some kernel C API functions return an "error pointer" which optionally
/// embeds an `errno`. Callers are supposed to check the returned pointer
/// for errors. This function performs the check and converts the "error pointer"
/// to a normal pointer in an idiomatic fashion.
///
/// # Examples
///
/// ```ignore
/// # use kernel::from_err_ptr;
/// # use kernel::bindings;
/// fn devm_platform_ioremap_resource(
///     pdev: &mut PlatformDevice,
///     index: u32,
/// ) -> Result<*mut core::ffi::c_void> {
///     // SAFETY: `pdev` points to a valid platform device. There are no safety requirements
///     // on `index`.
///     from_err_ptr(unsafe { bindings::devm_platform_ioremap_resource(pdev.to_ptr(), index) })
/// }
/// ```
// TODO: Remove `dead_code` marker once an in-kernel client is available.
#[allow(dead_code)]
pub fn from_err_ptr<T>(ptr: *mut T) -> KernelResult<*mut T> {
    // CAST: Casting a pointer to `*const core::ffi::c_void` is always valid.
    let const_ptr: *const core::ffi::c_void = ptr.cast();
    // SAFETY: The FFI function does not deref the pointer.
    if crate::sys_is_err(const_ptr) {
        // SAFETY: The FFI function does not deref the pointer.
        let err = crate::sys_ptr_err(const_ptr);
        // CAST: If `IS_ERR()` returns `true`,
        // then `PTR_ERR()` is guaranteed to return a
        // negative value greater-or-equal to `-bindings::MAX_ERRNO`,
        // which always fits in an `i16`, as per the invariant above.
        // And an `i16` always fits in an `i32`. So casting `err` to
        // an `i32` can never overflow, and is always valid.
        //
        // SAFETY: `IS_ERR()` ensures `err` is a
        // negative value greater-or-equal to `-bindings::MAX_ERRNO`.
        #[allow(clippy::unnecessary_cast)]
        return Err(unsafe { Error::from_errno_unchecked(err as core::ffi::c_int) });
    }
    Ok(ptr)
}

// Build-time error.
//
// This crate provides a [const function][const-functions] `build_error`, which will panic in
// compile-time if executed in [const context][const-context], and will cause a build error
// if not executed at compile time and the optimizer does not optimise away the call.
//
// It is used by `build_assert!` in the kernel crate, allowing checking of
// conditions that could be checked statically, but could not be enforced in
// Rust yet (e.g. perform some checks in [const functions][const-functions], but those
// functions could still be called in the runtime).
//
// For details on constant evaluation in Rust, please see the [Reference][const-eval].
//
// [const-eval]: https://doc.rust-lang.org/reference/const_eval.html
// [const-functions]: https://doc.rust-lang.org/reference/const_eval.html#const-functions
// [const-context]: https://doc.rust-lang.org/reference/const_eval.html#const-context

/// Panics if executed in [const context][const-context], or triggers a build error if not.
///
/// [const-context]: https://doc.rust-lang.org/reference/const_eval.html#const-context
#[inline(never)]
#[cold]
#[track_caller]
pub const fn build_error(msg: &'static str) -> ! {
    panic!("{}", msg);
}
