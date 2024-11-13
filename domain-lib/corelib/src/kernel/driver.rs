// SPDX-License-Identifier: GPL-2.0

//! Generic support for drivers of different buses (e.g., PCI, Platform, Amba, etc.).
//!
//! Each bus/subsystem is expected to implement [`DriverOps`], which allows drivers to register
//! using the [`DriverRegistration`] class.

use alloc::{boxed::Box, sync::Arc};
use core::{cell::UnsafeCell, marker::PhantomData, ops::Deref, pin::Pin};

use crate::kernel::{
    error::{linux_err::*, KernelResult as Result},
    str::CStr,
    ThisModule,
};

/// A subsystem (e.g., PCI, Platform, Amba, etc.) that allows drivers to be written for it.
pub trait DriverOps {
    /// The type that holds information about the registration. This is typically a struct defined
    /// by the C portion of the kernel.
    type RegType: Default;
    type DomainType;
    /// Registers a driver.
    ///
    /// # Safety
    ///
    /// `reg` must point to valid, initialised, and writable memory. It may be modified by this
    /// function to hold registration state.
    ///
    /// On success, `reg` must remain pinned and valid until the matching call to
    /// [`DriverOps::unregister`].
    unsafe fn register(
        reg: *mut Self::RegType,
        name: &'static CStr,
        module: ThisModule,
        domain: Self::DomainType,
    ) -> Result;

    /// Unregisters a driver previously registered with [`DriverOps::register`].
    ///
    /// # Safety
    ///
    /// `reg` must point to valid writable memory, initialised by a previous successful call to
    /// [`DriverOps::register`].
    unsafe fn unregister(reg: *mut Self::RegType);
}

/// The registration of a driver.
pub struct DriverRegistration<T: DriverOps> {
    is_registered: bool,
    concrete_reg: UnsafeCell<T::RegType>,
}

// SAFETY: `Registration` has no fields or methods accessible via `&Registration`, so it is safe to
// share references to it with multiple threads as nothing can be done.
unsafe impl<T: DriverOps> Sync for DriverRegistration<T> {}

impl<T: DriverOps> DriverRegistration<T> {
    /// Creates a new instance of the registration object.
    pub fn new() -> Self {
        Self {
            is_registered: false,
            concrete_reg: UnsafeCell::new(T::RegType::default()),
        }
    }

    /// Allocates a pinned registration object and registers it.
    ///
    /// Returns a pinned heap-allocated representation of the registration.
    pub fn new_pinned(
        name: &'static CStr,
        module: ThisModule,
        domain: T::DomainType,
    ) -> Result<Pin<Box<Self>>> {
        let mut reg = Pin::from(Box::try_new(Self::new())?);
        reg.as_mut().register(name, module, domain)?;
        Ok(reg)
    }

    /// Registers a driver with its subsystem.
    ///
    /// It must be pinned because the memory block that represents the registration is potentially
    /// self-referential.
    pub fn register(
        self: Pin<&mut Self>,
        name: &'static CStr,
        module: ThisModule,
        domain: T::DomainType,
    ) -> Result {
        // SAFETY: We never move out of `this`.
        let this = unsafe { self.get_unchecked_mut() };
        if this.is_registered {
            // Already registered.
            return Err(EINVAL);
        }

        // SAFETY: `concrete_reg` was initialised via its default constructor. It is only freed
        // after `Self::drop` is called, which first calls `T::unregister`.
        unsafe { T::register(this.concrete_reg.get(), name, module, domain) }?;

        this.is_registered = true;
        Ok(())
    }
}

impl<T: DriverOps> Default for DriverRegistration<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: DriverOps> Drop for DriverRegistration<T> {
    fn drop(&mut self) {
        if self.is_registered {
            // SAFETY: This path only runs if a previous call to `T::register` completed
            // successfully.
            unsafe { T::unregister(self.concrete_reg.get()) };
        }
    }
}

/// Conversion from a device id to a raw device id.
///
/// This is meant to be implemented by buses/subsystems so that they can use [`IdTable`] to
/// guarantee (at compile-time) zero-termination of device id tables provided by drivers.
///
/// # Safety
///
/// Implementers must ensure that:
///   - [`RawDeviceId::ZERO`] is actually a zeroed-out version of the raw device id.
///   - [`RawDeviceId::to_rawid`] stores `offset` in the context/data field of the raw device id so
///     that buses can recover the pointer to the data.
pub unsafe trait RawDeviceId {
    /// The raw type that holds the device id.
    ///
    /// Id tables created from [`Self`] are going to hold this type in its zero-terminated array.
    type RawType: Copy;

    /// A zeroed-out representation of the raw device id.
    ///
    /// Id tables created from [`Self`] use [`Self::ZERO`] as the sentinel to indicate the end of
    /// the table.
    const ZERO: Self::RawType;
}

/// A zero-terminated device id array, followed by context data.
#[repr(C)]
pub struct IdArray<T: RawDeviceId, U, const N: usize> {
    ids: [T::RawType; N],
    sentinel: T::RawType,
    id_infos: [Option<U>; N],
}

impl<T: RawDeviceId, U, const N: usize> IdArray<T, U, N> {
    const U_NONE: Option<U> = None;

    /// Returns an `IdTable` backed by `self`.
    ///
    /// This is used to essentially erase the array size.
    pub const fn as_table(&self) -> IdTable<'_, T, U> {
        IdTable {
            first: &self.ids[0],
            _p: PhantomData,
        }
    }

    /// Returns the number of items in the ID table.
    pub const fn count(&self) -> usize {
        self.ids.len()
    }

    /// Returns the inner IdArray array, without the context data.
    // pub const fn as_ids(&self) -> IdArrayIds<T, N>
    // where
    //     T: RawDeviceId + Copy,
    // {
    //     self.ids
    // }

    /// Creates a new instance of the array.
    ///
    /// The contents are derived from the given identifiers and context information.
    #[doc(hidden)]
    pub const unsafe fn new(raw_ids: [T::RawType; N], infos: [Option<U>; N]) -> Self
    where
        T: RawDeviceId + Copy,
        T::RawType: Copy + Clone,
    {
        Self {
            ids: raw_ids,
            sentinel: T::ZERO,
            id_infos: infos,
        }
    }

    #[doc(hidden)]
    pub const fn get_offset(idx: usize) -> isize
    where
        T: RawDeviceId + Copy,
        T::RawType: Copy + Clone,
    {
        // SAFETY: We are only using this dummy value to get offsets.
        let array = unsafe { Self::new([T::ZERO; N], [Self::U_NONE; N]) };
        // SAFETY: Both pointers are within `array` (or one byte beyond), consequently they are
        // derived from the same allocated object. We are using a `u8` pointer, whose size 1,
        // so the pointers are necessarily 1-byte aligned.
        let ret = unsafe {
            (&array.id_infos[idx] as *const _ as *const u8)
                .offset_from(&array.ids[idx] as *const _ as _)
        };
        core::mem::forget(array);
        ret
    }
}

// Creates a new ID array. This is a macro so it can take as a parameter the concrete ID type in order
// to call to_rawid() on it, and still remain const. This is necessary until a new const_trait_impl
// implementation lands, since the existing implementation was removed in Rust 1.73.
#[macro_export]
#[doc(hidden)]
macro_rules! _new_id_array {
    (($($args:tt)*), $id_type:ty) => {{
        /// Creates a new instance of the array.
        ///
        /// The contents are derived from the given identifiers and context information.
        const fn new< U, const N: usize>(ids: [$id_type; N], infos: [Option<U>; N])
            -> $crate::kernel::driver::IdArray<$id_type, U, N>
        where
            $id_type: $crate::kernel::driver::RawDeviceId + Copy,
            <$id_type as $crate::kernel::driver::RawDeviceId>::RawType: Copy + Clone,
        {
            let mut raw_ids =
                [<$id_type as $crate::kernel::driver::RawDeviceId>::ZERO; N];
            let mut i = 0usize;
            while i < N {
                let offset: isize = $crate::kernel::driver::IdArray::<$id_type, U, N>::get_offset(i);
                raw_ids[i] = ids[i].to_rawid(offset);
                i += 1;
            }

            // SAFETY: We are passing valid arguments computed with the correct offsets.
            unsafe {
                $crate::kernel::driver::IdArray::<$id_type, U, N>::new(raw_ids, infos)
            }
       }

        new($($args)*)
    }}
}

/// A device id table.
///
/// The table is guaranteed to be zero-terminated and to be followed by an array of context data of
/// type `Option<U>`.
pub struct IdTable<'a, T: RawDeviceId, U> {
    first: &'a T::RawType,
    _p: PhantomData<&'a U>,
}

impl<T: RawDeviceId, U> AsRef<T::RawType> for IdTable<'_, T, U> {
    fn as_ref(&self) -> &T::RawType {
        self.first
    }
}

#[macro_export]
macro_rules! count_paren_items {
    (($($item:tt)*), $($remaining:tt)*) => { 1 + $crate::count_paren_items!($($remaining)*) };
    (($($item:tt)*)) => { 1 };
    () => { 0 };
}

#[macro_export]
macro_rules! first_item {
    ($id_type:ty, $(({$($first:tt)*}, $second:expr)),* $(,)?) => {
        {
            type IdType = $id_type;
            [$(IdType{$($first)*},)*]
        }
    };
    ($id_type:ty, $(($first:expr, $second:expr)),* $(,)?) => { [$($first,)*] };
}

#[macro_export]
macro_rules! second_item {
    ($(({$($first:tt)*}, $second:expr)),* $(,)?) => { [$($second,)*] };
    ($(($first:expr, $second:expr)),* $(,)?) => { [$($second,)*] };
}

/// Defines a new constant [`IdArray`] with a concise syntax.
///
/// It is meant to be used by buses and subsystems to create a similar macro with their device id
/// type already specified, i.e., with fewer parameters to the end user.
///
/// # Examples
///
// TODO: Exported but not usable by kernel modules (requires `const_trait_impl`).
/// ```ignore
/// #![feature(const_trait_impl)]
/// # use kernel::{define_id_array, driver::RawDeviceId};
///
/// #[derive(Copy, Clone)]
/// struct Id(u32);
///
/// // SAFETY: `ZERO` is all zeroes and `to_rawid` stores `offset` as the second element of the raw
/// // device id pair.
/// unsafe impl const RawDeviceId for Id {
///     type RawType = (u64, isize);
///     const ZERO: Self::RawType = (0, 0);
///     fn to_rawid(&self, offset: isize) -> Self::RawType {
///         (self.0 as u64 + 1, offset)
///     }
/// }
///
/// define_id_array!(A1, Id, (), []);
/// define_id_array!(A2, Id, &'static [u8], [(Id(10), None)]);
/// define_id_array!(A3, Id, &'static [u8], [(Id(10), Some(b"id1")), ]);
/// define_id_array!(A4, Id, &'static [u8], [(Id(10), Some(b"id1")), (Id(20), Some(b"id2"))]);
/// define_id_array!(A5, Id, &'static [u8], [(Id(10), Some(b"id1")), (Id(20), Some(b"id2")), ]);
/// define_id_array!(A6, Id, &'static [u8], [(Id(10), None), (Id(20), Some(b"id2")), ]);
/// define_id_array!(A7, Id, &'static [u8], [(Id(10), Some(b"id1")), (Id(20), None), ]);
/// define_id_array!(A8, Id, &'static [u8], [(Id(10), None), (Id(20), None), ]);
/// ```
#[macro_export]
macro_rules! define_id_array {
    ($table_name:ident, $id_type:ty, $data_type:ty, [ $($t:tt)* ]) => {
        const $table_name:
            $crate::kernel::driver::IdArray<$id_type, $data_type, { $crate::count_paren_items!($($t)*) }> =
                $crate::_new_id_array!((
                    $crate::first_item!($id_type, $($t)*), $crate::second_item!($($t)*)), $id_type);
    };
}

/// Defines a new constant [`IdTable`] with a concise syntax.
///
/// It is meant to be used by buses and subsystems to create a similar macro with their device id
/// type already specified, i.e., with fewer parameters to the end user.
///
/// # Examples
///
// TODO: Exported but not usable by kernel modules (requires `const_trait_impl`).
/// ```ignore
/// #![feature(const_trait_impl)]
/// # use kernel::{define_id_table, driver::RawDeviceId};
///
/// #[derive(Copy, Clone)]
/// struct Id(u32);
///
/// // SAFETY: `ZERO` is all zeroes and `to_rawid` stores `offset` as the second element of the raw
/// // device id pair.
/// unsafe impl const RawDeviceId for Id {
///     type RawType = (u64, isize);
///     const ZERO: Self::RawType = (0, 0);
///     fn to_rawid(&self, offset: isize) -> Self::RawType {
///         (self.0 as u64 + 1, offset)
///     }
/// }
///
/// define_id_table!(T1, Id, &'static [u8], [(Id(10), None)]);
/// define_id_table!(T2, Id, &'static [u8], [(Id(10), Some(b"id1")), ]);
/// define_id_table!(T3, Id, &'static [u8], [(Id(10), Some(b"id1")), (Id(20), Some(b"id2"))]);
/// define_id_table!(T4, Id, &'static [u8], [(Id(10), Some(b"id1")), (Id(20), Some(b"id2")), ]);
/// define_id_table!(T5, Id, &'static [u8], [(Id(10), None), (Id(20), Some(b"id2")), ]);
/// define_id_table!(T6, Id, &'static [u8], [(Id(10), Some(b"id1")), (Id(20), None), ]);
/// define_id_table!(T7, Id, &'static [u8], [(Id(10), None), (Id(20), None), ]);
/// ```
#[macro_export]
macro_rules! define_id_table {
    ($table_name:ident, $id_type:ty, $data_type:ty, [ $($t:tt)* ]) => {
        const $table_name: Option<$crate::kernel::driver::IdTable<'static, $id_type, $data_type>> = {
            $crate::define_id_array!(ARRAY, $id_type, $data_type, [ $($t)* ]);
            Some(ARRAY.as_table())
        };
    };
}

/// Custom code within device removal.
pub trait DeviceRemoval {
    /// Cleans resources up when the device is removed.
    ///
    /// This is called when a device is removed and offers implementers the chance to run some code
    /// that cleans state up.
    fn device_remove(&self);
}

impl DeviceRemoval for () {
    fn device_remove(&self) {}
}

impl<T: DeviceRemoval> DeviceRemoval for Arc<T> {
    fn device_remove(&self) {
        self.deref().device_remove();
    }
}

impl<T: DeviceRemoval> DeviceRemoval for Box<T> {
    fn device_remove(&self) {
        self.deref().device_remove();
    }
}
