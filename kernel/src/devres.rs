use alloc::sync::Arc;
use core::{ffi::c_void, ops::Deref};

use pinned_init::{pin_data, pin_init, InPlaceInit};

use crate::{
    bindings,
    device::Device,
    error::{Error, KernelResult as Result},
    kalloc::AllocFlags,
    revocable::Revocable,
    UniqueArc,
};

#[pin_data]
struct DevresInner<T> {
    #[pin]
    data: Revocable<T>,
}

/// This abstraction is meant to be used by subsystems to containerize [`Device`] bound resources to
/// manage their lifetime.
///
/// [`Device`] bound resources should be freed when either the resource goes out of scope or the
/// [`Device`] is unbound respectively, depending on what happens first.
///
/// To achieve that [`Devres`] registers a devres callback on creation, which is called once the
/// [`Device`] is unbound, revoking access to the encapsulated resource (see also [`Revocable`]).
///
/// After the [`Devres`] has been unbound it is not possible to access the encapsulated resource
/// anymore.
///
/// [`Devres`] users should make sure to simply free the corresponding backing resource in `T`'s
/// [`Drop`] implementation.
///
/// # Example
///
/// ```
/// # use kernel::{bindings, c_str, device::Device, devres::Devres, mm::io_mem ::Io};
/// # use core::ops::Deref;
/// use kernel::code::{ENOMEM, ENXIO};
/// # use kernel::error::KernelResult as Result;///
/// use kernel::kalloc::flags::__GFP_ZERO;
///
/// // See also [`pci::Bar`] for a real example.
/// struct IoMem<const SIZE: usize>(Io<SIZE>);
///
/// impl<const SIZE: usize> IoMem<SIZE> {
///     fn new(paddr: usize) -> Result<Self>{
///
///         // SAFETY: assert safety for this example
///         let addr = unsafe { bindings::ioremap(paddr as _, SIZE.try_into().unwrap()) };
///         if addr.is_null() {
///             return Err(ENOMEM);
///         }
///
///         // SAFETY: `addr` is guaranteed to be the start of a valid I/O mapped memory region of
///         // size `SIZE`.
///         let io = unsafe { Io::new(addr as _, SIZE)? };
///
///         Ok(IoMem(io))
///     }
/// }
///
/// impl<const SIZE: usize> Drop for IoMem<SIZE> {
///     fn drop(&mut self) {
///         // SAFETY: Safe as by the invariant of `Io`.
///         unsafe { bindings::iounmap(self.0.base_addr() as _); };
///     }
/// }
///
/// impl<const SIZE: usize> Deref for IoMem<SIZE> {
///    type Target = Io<SIZE>;
///
///    fn deref(&self) -> &Self::Target {
///        &self.0
///    }
/// }
///
/// # // SAFETY: *NOT* safe, just for the example to get an `ARef<Device>` instance
/// # let dev = unsafe { Device::from_raw(core::ptr::null_mut()) };
///
/// let iomem = IoMem::<{ core::mem::size_of::<u32>() }>::new(0xBAAAAAAD).unwrap();
/// let devres = Devres::new(&dev, iomem, __GFP_ZERO).unwrap();
///
/// let res = devres.try_access().ok_or(ENXIO).unwrap();
/// res.writel(0x42, 0x0);
/// ```
///
pub struct Devres<T>(Arc<DevresInner<T>>);

impl<T> DevresInner<T> {
    fn new(dev: &Device, data: T, _flags: AllocFlags) -> Result<Arc<DevresInner<T>>> {
        let inner: Arc<DevresInner<T>> = UniqueArc::pin_init(pin_init!( DevresInner {
            data <- Revocable::new(data),
        }))?
        .into();

        // Convert `Arc<DevresInner>` into a raw pointer and make devres own this reference until
        // `Self::devres_callback` is called.
        let data = inner.clone();
        let data = Arc::into_raw(data);
        let ret = unsafe {
            bindings::devm_add_action(dev.as_raw(), Some(Self::devres_callback), data as _)
        };

        if ret != 0 {
            // SAFETY: We just created another reference to `inner` in order to pass it to
            // `bindings::devm_add_action`. If `bindings::devm_add_action` fails, we have to drop
            // this reference accordingly.
            let _ = unsafe { Arc::from_raw(data) };
            return Err(Error::from_errno(ret));
        }

        Ok(inner)
    }

    unsafe extern "C" fn devres_callback(ptr: *mut c_void) {
        let ptr = ptr as *mut DevresInner<T>;
        // Devres owned this memory; now that we received the callback, drop the `Arc` and hence the
        // reference.
        // SAFETY: Safe, since we leaked an `Arc` reference to devm_add_action() in
        //         `DevresInner::new`.
        let inner = unsafe { Arc::from_raw(ptr) };

        inner.data.revoke();
    }
}

impl<T> Devres<T> {
    /// Creates a new [`Devres`] instance of the given `data`. The `data` encapsulated within the
    /// returned `Devres` instance' `data` will be revoked once the device is detached.
    pub fn new(dev: &Device, data: T, flags: AllocFlags) -> Result<Self> {
        let inner = DevresInner::new(dev, data, flags)?;

        Ok(Devres(inner))
    }

    /// Same as [Devres::new`], but does not return a `Devres` instance. Instead the given `data`
    /// is owned by devres and will be revoked / dropped, once the device is detached.
    pub fn new_foreign_owned(dev: &Device, data: T, flags: AllocFlags) -> Result {
        let _ = DevresInner::new(dev, data, flags)?;

        Ok(())
    }
}

impl<T> Deref for Devres<T> {
    type Target = Revocable<T>;

    fn deref(&self) -> &Self::Target {
        &self.0.data
    }
}

impl<T> Drop for Devres<T> {
    fn drop(&mut self) {
        // Revoke the data, such that it gets dropped already and the actual resource is freed.
        // `DevresInner` has to stay alive until the devres callback has been called. This is
        // necessary since we don't know when `Devres` is dropped and calling
        // `devm_remove_action()` instead could race with `devres_release_all()`.
        self.revoke();
    }
}
