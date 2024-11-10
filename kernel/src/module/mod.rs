pub mod param;
use crate::{bindings, error, error::KernelResult as Result, init};

/// Prefix to appear before log messages printed from within the `kernel` crate.
const __LOG_PREFIX: &[u8] = b"rust_kernel\0";

/// The top level entrypoint to implementing a kernel module.
///
/// For any teardown or cleanup operations, your type may implement [`Drop`].
pub trait Module: Sized + Sync {
    /// Called at module initialization time.
    ///
    /// Use this method to perform whatever setup or registration your module
    /// should do.
    ///
    /// Equivalent to the `module_init` macro in the C API.
    fn init(module: &'static ThisModule) -> Result<Self>;
}

/// A module that is pinned and initialised in-place.
pub trait InPlaceModule: Sync {
    /// Creates an initialiser for the module.
    ///
    /// It is called when the module is loaded.
    fn init(module: &'static ThisModule) -> impl init::PinInit<Self, error::Error>;
}

impl<T: Module> InPlaceModule for T {
    fn init(module: &'static ThisModule) -> impl init::PinInit<Self, error::Error> {
        let initer = move |slot: *mut Self| {
            let m = <Self as Module>::init(module)?;

            // SAFETY: `slot` is valid for write per the contract with `pin_init_from_closure`.
            unsafe { slot.write(m) };
            Ok(())
        };

        // SAFETY: On success, `initer` always fully initialises an instance of `Self`.
        unsafe { init::pin_init_from_closure(initer) }
    }
}

/// Equivalent to `THIS_MODULE` in the C API.
///
/// C header: `include/linux/export.h`
pub struct ThisModule(pub(crate) *mut bindings::module);

// SAFETY: `THIS_MODULE` may be used from all threads within a module.
unsafe impl Sync for ThisModule {}

impl ThisModule {
    /// Creates a [`ThisModule`] given the `THIS_MODULE` pointer.
    ///
    /// # Safety
    ///
    /// The pointer must be equal to the right `THIS_MODULE`.
    pub const unsafe fn from_ptr(ptr: *mut bindings::module) -> ThisModule {
        ThisModule(ptr)
    }

    pub fn as_ptr(&self) -> *mut bindings::module {
        self.0
    }
}
