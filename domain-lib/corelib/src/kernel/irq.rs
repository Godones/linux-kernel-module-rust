// SPDX-License-Identifier: GPL-2.0

//! IRQ Abstractions

#![allow(dead_code)]

use alloc::{boxed::Box, sync::Arc};
use core::{fmt, marker::PhantomData};

use interface::nvme::IrqHandlerOp;
use kbind::safe_ptr::SafePtr;

use crate::{
    bindings,
    kernel::{
        error::{to_result, KernelResult as Result},
        str::CString,
        types::{ForeignOwnable, ScopeGuard},
    },
};

struct InternalRegistration<T: ForeignOwnable, H: IrqHandler> {
    irq: u32,
    data_shim: *mut core::ffi::c_void,
    name: CString,
    _p: PhantomData<T>,
    _h: PhantomData<H>,
}

impl<T: ForeignOwnable, H: IrqHandler> InternalRegistration<T, H> {
    /// Registers a new irq handler.
    ///
    /// # Safety
    ///
    /// Callers must ensure that `handler` and `thread_fn` are compatible with the registration,
    /// that is, that they only use their second argument while the call is happening and that they
    /// only call [`T::borrow`] on it (e.g., they shouldn't call [`T::from_foreign`] and consume
    /// it).
    unsafe fn try_new(
        irq: core::ffi::c_uint,
        handler: bindings::irq_handler_t,
        thread_fn: bindings::irq_handler_t,
        flags: usize,
        data: T,
        name: fmt::Arguments<'_>,
        domain: Arc<dyn IrqHandlerOp>,
    ) -> Result<Self> {
        let data_ptr = data.into_foreign() as *mut _;
        let name = CString::try_from_fmt(name)?;
        let guard = ScopeGuard::new(|| {
            // SAFETY: `ptr` came from a previous call to `into_foreign`.
            unsafe { T::from_foreign(data_ptr) };
        });

        let data_shim = IrqHandlerDataShim::new(data_ptr, domain);
        let data_shim = Box::into_raw(Box::new(data_shim));
        // SAFETY: `name` and `ptr` remain valid as long as the registration is alive.
        to_result(crate::sys_request_threaded_irq(
            irq,
            handler,
            thread_fn,
            flags as _,
            name.as_char_ptr(),
            data_shim as _,
        ))?;
        guard.dismiss();
        Ok(Self {
            irq,
            name,
            data_shim: data_shim as _,
            _p: PhantomData,
            _h: PhantomData,
        })
    }
}

impl<T: ForeignOwnable, H: IrqHandler> Drop for InternalRegistration<T, H> {
    fn drop(&mut self) {
        // Unregister irq handler.
        //
        // SAFETY: When `try_new` succeeds, the irq was successfully requested, so it is ok to free
        // it here.
        crate::sys_free_irq(self.irq, self.data_shim);

        // Free context data.
        //
        // SAFETY: This matches the call to `into_foreign` from `try_new` in the success case.
        let data_shim = unsafe { Box::from_raw(self.data_shim as *mut IrqHandlerDataShim) };
        unsafe { T::from_foreign(data_shim.data) };
    }
}

/// An irq handler.
pub trait IrqHandler {
    // The context data associated with and made available to the handler.
    type Data: ForeignOwnable;
    /// Called from interrupt context when the irq happens.
    fn handle_irq(data: SafePtr) -> Return;
}

pub struct IrqRegistration<H: IrqHandler>(InternalRegistration<H::Data, H>);

impl<H: IrqHandler> IrqRegistration<H> {
    /// Registers a new irq handler.
    ///
    /// The valid values of `flags` come from the [`flags`] module.
    pub fn try_new(
        irq: u32,
        data: H::Data,
        flags: usize,
        name: fmt::Arguments<'_>,
        domain: Arc<dyn IrqHandlerOp>,
    ) -> Result<Self> {
        // SAFETY: `handler` only calls `H::Data::borrow` on `raw_data`.
        Ok(Self(unsafe {
            InternalRegistration::try_new(
                irq,
                Some(shim::handler),
                None,
                flags,
                data,
                name,
                domain,
            )?
        }))
    }

    // unsafe extern "C" fn handler(
    //     _irq: core::ffi::c_int,
    //     raw_data: *mut core::ffi::c_void,
    // ) -> bindings::irqreturn_t {
    //     // SAFETY: On registration, `into_foreign` was called, so it is safe to borrow from it here
    //     // because `from_foreign` is called only after the irq is unregistered.
    //     let data = unsafe { H::Data::borrow(raw_data) };
    //     H::handle_irq(data) as _
    // }
}

pub struct IrqHandlerDataShim {
    pub data: *mut core::ffi::c_void,
    pub domain: Arc<dyn IrqHandlerOp>,
}

impl IrqHandlerDataShim {
    pub fn new(data: *mut core::ffi::c_void, domain: Arc<dyn IrqHandlerOp>) -> Self {
        Self { data, domain }
    }
    pub fn from_foreign(data: *mut core::ffi::c_void) -> &'static IrqHandlerDataShim {
        // SAFETY: `data` is a valid pointer to `IrqHandlerDataShim`.
        unsafe { &*(data as *const IrqHandlerDataShim) }
    }
    pub fn domain(&self) -> &Arc<dyn IrqHandlerOp> {
        &self.domain
    }
}

pub use shim::IrqHandlerShim;

mod shim {
    use kbind::safe_ptr::SafePtr;

    use super::{IrqHandler, IrqHandlerDataShim};
    use crate::{
        bindings,
        kernel::{error::KernelResult as Result, types::ForeignOwnable},
    };

    pub unsafe extern "C" fn handler(
        _irq: core::ffi::c_int,
        raw_data: *mut core::ffi::c_void,
    ) -> bindings::irqreturn_t {
        let data_shim = IrqHandlerDataShim::from_foreign(raw_data);
        let domain = data_shim.domain();
        let res = domain
            .handle_irq(SafePtr::new(data_shim.data))
            .expect("irq handler failed");
        res
    }

    pub struct IrqHandlerShim<T: IrqHandler>(core::marker::PhantomData<T>);

    impl<T: IrqHandler> IrqHandlerShim<T> {
        pub fn handle_irq(raw_data: SafePtr) -> Result<u32> {
            let res =
                // let raw_data = raw_data.raw_ptr();
                // let data = T::Data::borrow(raw_data) ;
                T::handle_irq(raw_data);
            Ok(res as u32)
        }
    }
}

/// The return value from interrupt handlers.
pub enum Return {
    /// The interrupt was not from this device or was not handled.
    None = bindings::irqreturn_IRQ_NONE as _,

    /// The interrupt was handled by this device.
    Handled = bindings::irqreturn_IRQ_HANDLED as _,

    /// The handler wants the handler thread to wake up.
    WakeThread = bindings::irqreturn_IRQ_WAKE_THREAD as _,
}

/// Container for interrupt flags.
pub mod flags {
    use crate::bindings;

    /// Use the interrupt line as already configured.
    pub const TRIGGER_NONE: usize = bindings::IRQF_TRIGGER_NONE as _;

    /// The interrupt is triggered when the signal goes from low to high.
    pub const TRIGGER_RISING: usize = bindings::IRQF_TRIGGER_RISING as _;

    /// The interrupt is triggered when the signal goes from high to low.
    pub const TRIGGER_FALLING: usize = bindings::IRQF_TRIGGER_FALLING as _;

    /// The interrupt is triggered while the signal is held high.
    pub const TRIGGER_HIGH: usize = bindings::IRQF_TRIGGER_HIGH as _;

    /// The interrupt is triggered while the signal is held low.
    pub const TRIGGER_LOW: usize = bindings::IRQF_TRIGGER_LOW as _;

    /// Allow sharing the irq among several devices.
    pub const SHARED: usize = bindings::IRQF_SHARED as _;

    /// Set by callers when they expect sharing mismatches to occur.
    pub const PROBE_SHARED: usize = bindings::IRQF_PROBE_SHARED as _;

    /// Flag to mark this interrupt as timer interrupt.
    pub const TIMER: usize = bindings::IRQF_TIMER as _;

    /// Interrupt is per cpu.
    pub const PERCPU: usize = bindings::IRQF_PERCPU as _;

    /// Flag to exclude this interrupt from irq balancing.
    pub const NOBALANCING: usize = bindings::IRQF_NOBALANCING as _;

    /// Interrupt is used for polling (only the interrupt that is registered first in a shared
    /// interrupt is considered for performance reasons).
    pub const IRQPOLL: usize = bindings::IRQF_IRQPOLL as _;

    /// Interrupt is not reenabled after the hardirq handler finished. Used by threaded interrupts
    /// which need to keep the irq line disabled until the threaded handler has been run.
    pub const ONESHOT: usize = bindings::IRQF_ONESHOT as _;

    /// Do not disable this IRQ during suspend. Does not guarantee that this interrupt will wake
    /// the system from a suspended state.
    pub const NO_SUSPEND: usize = bindings::IRQF_NO_SUSPEND as _;

    /// Force enable it on resume even if [`NO_SUSPEND`] is set.
    pub const FORCE_RESUME: usize = bindings::IRQF_FORCE_RESUME as _;

    /// Interrupt cannot be threaded.
    pub const NO_THREAD: usize = bindings::IRQF_NO_THREAD as _;

    /// Resume IRQ early during syscore instead of at device resume time.
    pub const EARLY_RESUME: usize = bindings::IRQF_EARLY_RESUME as _;

    /// If the IRQ is shared with a NO_SUSPEND user, execute this interrupt handler after
    /// suspending interrupts. For system wakeup devices users need to implement wakeup detection
    /// in their interrupt handlers.
    pub const COND_SUSPEND: usize = bindings::IRQF_COND_SUSPEND as _;

    /// Don't enable IRQ or NMI automatically when users request it. Users will enable it
    /// explicitly by `enable_irq` or `enable_nmi` later.
    pub const NO_AUTOEN: usize = bindings::IRQF_NO_AUTOEN as _;

    /// Exclude from runnaway detection for IPI and similar handlers, depends on `PERCPU`.
    pub const NO_DEBUG: usize = bindings::IRQF_NO_DEBUG as _;
}
