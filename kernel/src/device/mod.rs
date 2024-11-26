// SPDX-License-Identifier: GPL-2.0
//

//! Generic devices that are part of the kernel's driver model.
//!
//! C header: [`include/linux/device.h`](../../../../include/linux/device.h)

use core::{
    fmt,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr,
};

use pinned_init::{pin_data, pin_init};

use crate::{
    bindings, c_str,
    error::{linux_err::*, Error, KernelResult as Result},
    init::{InPlaceInit, PinInit},
    revocable::{Revocable, RevocableGuard},
    str::CStr,
    sync::{LockClassKey, RevocableMutex},
    types::{ARef, Opaque},
    UniqueArc,
};

pub(crate) mod block;

/// A reference-counted device.
///
/// This structure represents the Rust abstraction for a C `struct device`. This implementation
/// abstracts the usage of an already existing C `struct device` within Rust code that we get
/// passed from the C side.
///
/// An instance of this abstraction can be obtained temporarily or permanent.
///
/// A temporary one is bound to the lifetime of the C `struct device` pointer used for creation.
/// A permanent instance is always reference-counted and hence not restricted by any lifetime
/// boundaries.
///
/// For subsystems it is recommended to create a permanent instance to wrap into a subsystem
/// specific device structure (e.g. `pci::Device`). This is useful for passing it to drivers in
/// `T::probe()`, such that a driver can store the `ARef<Device>` (equivalent to storing a
/// `struct device` pointer in a C driver) for arbitrary purposes, e.g. allocating DMA coherent
/// memory.
///
/// # Invariants
///
/// A `Device` instance represents a valid `struct device` created by the C portion of the kernel.
///
/// Instances of this type are always reference-counted, that is, a call to `get_device` ensures
/// that the allocation remains valid at least until the matching call to `put_device`.
///
/// `bindings::device::release` is valid to be called from any thread, hence `ARef<Device>` can be
/// dropped from any thread.
#[repr(transparent)]
pub struct Device(Opaque<bindings::device>);

// SAFETY: `Device` only holds a pointer to a C device, which is safe to be used from any thread.
unsafe impl Send for Device {}

// SAFETY: `Device` only holds a pointer to a C device, references to which are safe to be used
// from any thread.
unsafe impl Sync for Device {}

impl Device {
    /// Creates a new reference-counted abstraction instance of an existing `struct device` pointer.
    ///
    /// # Safety
    ///
    /// Callers must ensure that `ptr` is valid, non-null, and has a non-zero reference count,
    /// i.e. it must be ensured that the reference count of the C `struct device` `ptr` points to
    /// can't drop to zero, for the duration of this function call.
    ///
    /// It must also be ensured that `bindings::device::release` can be called from any thread.
    /// While not officially documented, this should be the case for any `struct device`.
    pub unsafe fn from_raw(ptr: *mut bindings::device) -> ARef<Self> {
        // SAFETY: By the safety requirements, ptr is valid.
        // Initially increase the reference count by one to compensate for the final decrement once
        // this newly created `ARef<Device>` instance is dropped.
        unsafe { bindings::get_device(ptr) };

        // CAST: `Self` is a `repr(transparent)` wrapper around `bindings::device`.
        let ptr = ptr.cast::<Self>();

        // SAFETY: `ptr` is valid by the safety requirements of this function. By the above call to
        // `bindings::get_device` we also own a reference to the underlying `struct device`.
        unsafe { ARef::from_raw(ptr::NonNull::new_unchecked(ptr)) }
    }
    /// Obtain the raw `struct device *`.
    pub fn as_raw(&self) -> *mut bindings::device {
        self.0.get()
    }

    /// Convert a raw C `struct device` pointer to a `&'a Device`.
    ///
    /// # Safety
    ///
    /// Callers must ensure that `ptr` is valid, non-null, and has a non-zero reference count,
    /// i.e. it must be ensured that the reference count of the C `struct device` `ptr` points to
    /// can't drop to zero, for the duration of this function call and the entire duration when the
    /// returned reference exists.
    pub unsafe fn as_ref<'a>(ptr: *mut bindings::device) -> &'a Self {
        // SAFETY: Guaranteed by the safety requirements of the function.
        unsafe { &*ptr.cast() }
    }

    /// Prints an emergency-level message (level 0) prefixed with device information.
    ///
    /// More details are available from [`dev_emerg`].
    ///
    /// [`dev_emerg`]: crate::dev_emerg
    pub fn pr_emerg(&self, args: fmt::Arguments<'_>) {
        // SAFETY: `klevel` is null-terminated, uses one of the kernel constants.
        unsafe { self.printk(bindings::KERN_EMERG, args) };
    }

    /// Prints an alert-level message (level 1) prefixed with device information.
    ///
    /// More details are available from [`dev_alert`].
    ///
    /// [`dev_alert`]: crate::dev_alert
    pub fn pr_alert(&self, args: fmt::Arguments<'_>) {
        // SAFETY: `klevel` is null-terminated, uses one of the kernel constants.
        unsafe { self.printk(bindings::KERN_ALERT, args) };
    }

    /// Prints a critical-level message (level 2) prefixed with device information.
    ///
    /// More details are available from [`dev_crit`].
    ///
    /// [`dev_crit`]: crate::dev_crit
    pub fn pr_crit(&self, args: fmt::Arguments<'_>) {
        // SAFETY: `klevel` is null-terminated, uses one of the kernel constants.
        unsafe { self.printk(bindings::KERN_CRIT, args) };
    }

    /// Prints an error-level message (level 3) prefixed with device information.
    ///
    /// More details are available from [`dev_err`].
    ///
    /// [`dev_err`]: crate::dev_err
    pub fn pr_err(&self, args: fmt::Arguments<'_>) {
        // SAFETY: `klevel` is null-terminated, uses one of the kernel constants.
        unsafe { self.printk(bindings::KERN_ERR, args) };
    }

    /// Prints a warning-level message (level 4) prefixed with device information.
    ///
    /// More details are available from [`dev_warn`].
    ///
    /// [`dev_warn`]: crate::dev_warn
    pub fn pr_warn(&self, args: fmt::Arguments<'_>) {
        // SAFETY: `klevel` is null-terminated, uses one of the kernel constants.
        unsafe { self.printk(bindings::KERN_WARNING, args) };
    }

    /// Prints a notice-level message (level 5) prefixed with device information.
    ///
    /// More details are available from [`dev_notice`].
    ///
    /// [`dev_notice`]: crate::dev_notice
    pub fn pr_notice(&self, args: fmt::Arguments<'_>) {
        // SAFETY: `klevel` is null-terminated, uses one of the kernel constants.
        unsafe { self.printk(bindings::KERN_NOTICE, args) };
    }

    /// Prints an info-level message (level 6) prefixed with device information.
    ///
    /// More details are available from [`dev_info`].
    ///
    /// [`dev_info`]: crate::dev_info
    pub fn pr_info(&self, args: fmt::Arguments<'_>) {
        // SAFETY: `klevel` is null-terminated, uses one of the kernel constants.
        unsafe { self.printk(bindings::KERN_INFO, args) };
    }

    /// Prints a debug-level message (level 7) prefixed with device information.
    ///
    /// More details are available from [`dev_dbg`].
    ///
    /// [`dev_dbg`]: crate::dev_dbg
    pub fn pr_dbg(&self, args: fmt::Arguments<'_>) {
        if cfg!(debug_assertions) {
            // SAFETY: `klevel` is null-terminated, uses one of the kernel constants.
            unsafe { self.printk(bindings::KERN_DEBUG, args) };
        }
    }
    /// Prints the provided message to the console.
    ///
    /// # Safety
    ///
    /// Callers must ensure that `klevel` is null-terminated; in particular, one of the
    /// `KERN_*`constants, for example, `KERN_CRIT`, `KERN_ALERT`, etc.
    unsafe fn printk(&self, klevel: &[u8], msg: fmt::Arguments<'_>) {
        // SAFETY: `klevel` is null-terminated and one of the kernel constants. `self.as_raw`
        // is valid because `self` is valid. The "%pA" format string expects a pointer to
        // `fmt::Arguments`, which is what we're passing as the last argument.
        unsafe {
            bindings::_dev_printk(
                klevel as *const _ as *const core::ffi::c_char,
                self.as_raw(),
                c_str!("%pA").as_char_ptr(),
                &msg as *const _ as *const core::ffi::c_void,
            )
        };
    }
    pub fn dma_set_mask(&self, mask: u64) -> Result {
        let dev = self.as_raw();
        let ret = unsafe { bindings::dma_set_mask(dev as _, mask) };
        if ret != 0 {
            Err(Error::from_errno(ret))
        } else {
            Ok(())
        }
    }
    pub fn dma_set_coherent_mask(&self, mask: u64) -> Result {
        let dev = self.as_raw();
        let ret = unsafe { bindings::dma_set_coherent_mask(dev as _, mask) };
        if ret != 0 {
            Err(Error::from_errno(ret))
        } else {
            Ok(())
        }
    }
    pub fn dma_map_sg(&self, sglist: &mut [bindings::scatterlist], dir: u32) -> Result {
        let dev = self.as_raw();
        let count = sglist.len().try_into()?;
        let ret = unsafe {
            bindings::dma_map_sg_attrs(
                dev,
                &mut sglist[0],
                count,
                dir,
                bindings::DMA_ATTR_NO_WARN.into(),
            )
        };
        // TODO: It may map fewer than what was requested. What happens then?
        if ret == 0 {
            return Err(EIO);
        }
        Ok(())
    }

    pub fn dma_unmap_sg(&self, sglist: &mut [bindings::scatterlist], dir: u32) {
        let dev = self.as_raw();
        let count = sglist.len() as _;
        unsafe { bindings::dma_unmap_sg_attrs(dev, &mut sglist[0], count, dir, 0) };
    }
}

// SAFETY: Instances of `Device` are always reference-counted.
unsafe impl crate::types::AlwaysRefCounted for Device {
    fn inc_ref(&self) {
        // SAFETY: The existence of a shared reference guarantees that the refcount is non-zero.
        unsafe { bindings::get_device(self.as_raw()) };
    }

    unsafe fn dec_ref(obj: ptr::NonNull<Self>) {
        // SAFETY: The safety requirements guarantee that the refcount is non-zero.
        unsafe { bindings::put_device(obj.cast().as_ptr()) }
    }
}

#[doc(hidden)]
#[macro_export]
macro_rules! dev_printk {
    ($method:ident, $dev:expr, $($f:tt)*) => {
        {
            ($dev).$method(core::format_args!($($f)*));
        }
    }
}

/// Prints an emergency-level message (level 0) prefixed with device information.
///
/// This level should be used if the system is unusable.
///
/// Equivalent to the kernel's `dev_emerg` macro.
///
/// Mimics the interface of [`std::print!`]. More information about the syntax is available from
/// [`core::fmt`] and [`alloc::format!`].
///
/// [`std::print!`]: https://doc.rust-lang.org/std/macro.print.html
///
/// # Examples
///
/// ```
/// # use kernel::dev_emerg;
/// use kernel::device::Device;
///
/// fn example(dev: &Device) {
///     dev_emerg!(dev, "hello {}\n", "there");
/// }
/// ```
#[macro_export]
macro_rules! dev_emerg {
    ($($f:tt)*) => { $crate::dev_printk!(pr_emerg, $($f)*); }
}

/// Prints an alert-level message (level 1) prefixed with device information.
///
/// This level should be used if action must be taken immediately.
///
/// Equivalent to the kernel's `dev_alert` macro.
///
/// Mimics the interface of [`std::print!`]. More information about the syntax is available from
/// [`core::fmt`] and [`alloc::format!`].
///
/// [`std::print!`]: https://doc.rust-lang.org/std/macro.print.html
///
/// # Examples
///
/// ```
/// # use kernel::dev_alert;
/// use kernel::device::Device;
///
/// fn example(dev: &Device) {
///     dev_alert!(dev, "hello {}\n", "there");
/// }
/// ```
#[macro_export]
macro_rules! dev_alert {
    ($($f:tt)*) => { $crate::dev_printk!(pr_alert, $($f)*); }
}

/// Prints a critical-level message (level 2) prefixed with device information.
///
/// This level should be used in critical conditions.
///
/// Equivalent to the kernel's `dev_crit` macro.
///
/// Mimics the interface of [`std::print!`]. More information about the syntax is available from
/// [`core::fmt`] and [`alloc::format!`].
///
/// [`std::print!`]: https://doc.rust-lang.org/std/macro.print.html
///
/// # Examples
///
/// ```
/// # use kernel::dev_crit;
/// use kernel::device::Device;
///
/// fn example(dev: &Device) {
///     dev_crit!(dev, "hello {}\n", "there");
/// }
/// ```
#[macro_export]
macro_rules! dev_crit {
    ($($f:tt)*) => { $crate::dev_printk!(pr_crit, $($f)*); }
}

/// Prints an error-level message (level 3) prefixed with device information.
///
/// This level should be used in error conditions.
///
/// Equivalent to the kernel's `dev_err` macro.
///
/// Mimics the interface of [`std::print!`]. More information about the syntax is available from
/// [`core::fmt`] and [`alloc::format!`].
///
/// [`std::print!`]: https://doc.rust-lang.org/std/macro.print.html
///
/// # Examples
///
/// ```
/// # use kernel::dev_err;
/// use kernel::device::Device;
///
/// fn example(dev: &Device) {
///     dev_err!(dev, "hello {}\n", "there");
/// }
/// ```
#[macro_export]
macro_rules! dev_err {
    ($($f:tt)*) => { $crate::dev_printk!(pr_err, $($f)*); }
}

/// Prints a warning-level message (level 4) prefixed with device information.
///
/// This level should be used in warning conditions.
///
/// Equivalent to the kernel's `dev_warn` macro.
///
/// Mimics the interface of [`std::print!`]. More information about the syntax is available from
/// [`core::fmt`] and [`alloc::format!`].
///
/// [`std::print!`]: https://doc.rust-lang.org/std/macro.print.html
///
/// # Examples
///
/// ```
/// # use kernel::dev_warn;
/// use kernel::device::Device;
///
/// fn example(dev: &Device) {
///     dev_warn!(dev, "hello {}\n", "there");
/// }
/// ```
#[macro_export]
macro_rules! dev_warn {
    ($($f:tt)*) => { $crate::dev_printk!(pr_warn, $($f)*); }
}

/// Prints a notice-level message (level 5) prefixed with device information.
///
/// This level should be used in normal but significant conditions.
///
/// Equivalent to the kernel's `dev_notice` macro.
///
/// Mimics the interface of [`std::print!`]. More information about the syntax is available from
/// [`core::fmt`] and [`alloc::format!`].
///
/// [`std::print!`]: https://doc.rust-lang.org/std/macro.print.html
///
/// # Examples
///
/// ```
/// # use kernel::dev_notice;
/// use kernel::device::Device;
///
/// fn example(dev: &Device) {
///     dev_notice!(dev, "hello {}\n", "there");
/// }
/// ```
#[macro_export]
macro_rules! dev_notice {
    ($($f:tt)*) => { $crate::dev_printk!(pr_notice, $($f)*); }
}

/// Prints an info-level message (level 6) prefixed with device information.
///
/// This level should be used for informational messages.
///
/// Equivalent to the kernel's `dev_info` macro.
///
/// Mimics the interface of [`std::print!`]. More information about the syntax is available from
/// [`core::fmt`] and [`alloc::format!`].
///
/// [`std::print!`]: https://doc.rust-lang.org/std/macro.print.html
///
/// # Examples
///
/// ```
/// # use kernel::dev_info;
/// use kernel::device::Device;
///
/// fn example(dev: &Device) {
///     dev_info!(dev, "hello {}\n", "there");
/// }
/// ```
#[macro_export]
macro_rules! dev_info {
    ($($f:tt)*) => { $crate::dev_printk!(pr_info, $($f)*); }
}

/// Prints a debug-level message (level 7) prefixed with device information.
///
/// This level should be used for debug messages.
///
/// Equivalent to the kernel's `dev_dbg` macro, except that it doesn't support dynamic debug yet.
///
/// Mimics the interface of [`std::print!`]. More information about the syntax is available from
/// [`core::fmt`] and [`alloc::format!`].
///
/// [`std::print!`]: https://doc.rust-lang.org/std/macro.print.html
///
/// # Examples
///
/// ```
/// # use kernel::dev_dbg;
/// use kernel::device::Device;
///
/// fn example(dev: &Device) {
///     dev_dbg!(dev, "hello {}\n", "there");
/// }
/// ```
#[macro_export]
macro_rules! dev_dbg {
    ($($f:tt)*) => { $crate::dev_printk!(pr_dbg, $($f)*); }
}

/// Device data.
///
/// When a device is removed (for whatever reason, for example, because the device was unplugged or
/// because the user decided to unbind the driver), the driver is given a chance to clean its state
/// up, and all io resources should ideally not be used anymore.
///
/// However, the device data is reference-counted because other subsystems hold pointers to it. So
/// some device state must be freed and not used anymore, while others must remain accessible.
///
/// This struct separates the device data into three categories:
///   1. Registrations: are destroyed when the device is removed, but before the io resources
///      become inaccessible.
///   2. Io resources: are available until the device is removed.
///   3. General data: remain available as long as the ref count is nonzero.
///
/// This struct implements the `DeviceRemoval` trait so that it can clean resources up even if not
/// explicitly called by the device drivers.
#[pin_data]
pub struct Data<T, U, V> {
    #[pin]
    registrations: RevocableMutex<T>,
    #[pin]
    resources: Revocable<U>,
    #[pin]
    general: V,
}

/// Safely creates an new reference-counted instance of [`Data`].
#[doc(hidden)]
#[macro_export]
macro_rules! new_device_data {
    ($reg:expr, $res:expr, $gen:expr, $name:literal) => {{
        static CLASS1: $crate::sync::LockClassKey = $crate::sync::LockClassKey::new();
        let regs = $reg;
        let res = $res;
        let gen = $gen;
        let name = $crate::c_str!($name);
        $crate::device::Data::try_new(regs, res, gen, name, &CLASS1)
    }};
}

impl<T, U, V> Data<T, U, V> {
    /// Creates a new instance of `Data`.
    ///
    /// It is recommended that the [`new_device_data`] macro be used as it automatically creates
    /// the lock classes.
    pub fn try_new(
        registrations: T,
        resources: impl PinInit<U>,
        general: impl PinInit<V>,
        name: &'static CStr,
        key1: &'static LockClassKey,
    ) -> Result<Pin<UniqueArc<Self>>> {
        let ret = UniqueArc::pin_init(pin_init!(Self {
            registrations <- RevocableMutex::new(
                registrations,
                name,
                key1,
            ),
            resources <- Revocable::new(resources),
            general <- general,
        }))?;

        Ok(ret)
    }

    /// Returns the resources if they're still available.
    pub fn resources(&self) -> Option<RevocableGuard<'_, U>> {
        self.resources.try_access()
    }

    // Returns the locked registrations if they're still available.
    // #[cfg(disabled)]
    // pub fn registrations(&self) -> Option<RevocableMutexGuard<'_, T>> {
    //     self.registrations.try_write()
    // }
}

impl<T, U, V> crate::driver::DeviceRemoval for Data<T, U, V> {
    fn device_remove(&self) {
        // We revoke the registrations first so that resources are still available to them during
        // unregistration.
        self.registrations.revoke();

        // Release resources now. General data remains available.
        self.resources.revoke();
    }
}

impl<T, U, V> Deref for Data<T, U, V> {
    type Target = V;

    fn deref(&self) -> &V {
        &self.general
    }
}

impl<T, U, V> DerefMut for Data<T, U, V> {
    fn deref_mut(&mut self) -> &mut V {
        &mut self.general
    }
}
