// SPDX-License-Identifier: GPL-2.0

//! Generic support for drivers of different buses (e.g., PCI, Platform, Amba, etc.).
//!
//! Each bus/subsystem is expected to implement [`DriverOps`], which allows drivers to register
//! using the [`DriverRegistration`] class.

use alloc::{boxed::Box, sync::Arc};
use core::{marker::PhantomData, ops::Deref, pin::Pin};

use pinned_init::{pin_data, pinned_drop, try_pin_init, PinInit};

use crate::{
    error::{Error, KernelResult as Result},
    str::CStr,
    types::Opaque,
    ThisModule,
};

/// A subsystem (e.g., PCI, Platform, Amba, etc.) that allows drivers to be written for it.
pub trait DriverOps {
    /// The type that holds information about the registration. This is typically a struct defined
    /// by the C portion of the kernel.
    type RegType: Default;

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
        module: &'static ThisModule,
    ) -> Result;

    /// Unregisters a driver previously registered with [`DriverOps::register`].
    ///
    /// # Safety
    ///
    /// `reg` must point to valid writable memory, initialised by a previous successful call to
    /// [`DriverOps::register`].
    fn unregister(reg: &mut Self::RegType);
}

/// A [`Registration`] is a generic type that represents the registration of some driver type (e.g.
/// `bindings::pci_driver`). Therefore a [`Registration`] is initialized with some type that
/// implements the [`DriverOps`] trait, such that the generic `T::register` and `T::unregister`
/// calls result in the subsystem specific registration calls.
///
///Once the `Registration` structure is dropped, the driver is unregistered.
#[pin_data(PinnedDrop)]
pub struct DriverRegistration<T: DriverOps> {
    #[pin]
    reg: Opaque<T::RegType>,
}

// SAFETY: `Registration` has no fields or methods accessible via `&Registration`, so it is safe to
// share references to it with multiple threads as nothing can be done.
unsafe impl<T: DriverOps> Sync for DriverRegistration<T> {}

impl<T: DriverOps> DriverRegistration<T> {
    /// Creates a new instance of the registration object.
    pub fn new(name: &'static CStr, module: &'static ThisModule) -> impl PinInit<Self, Error> {
        let res = try_pin_init!(Self {
            reg <- Opaque::try_ffi_init(|ptr: *mut T::RegType| unsafe{
                // SAFETY: `try_ffi_init` guarantees that `ptr` is valid for write.
                ptr.write(T::RegType::default()) ;

                // SAFETY: `try_ffi_init` guarantees that `ptr` is valid for write, and it has
                // just been initialised above, so it's also valid for read.
                let drv = &mut *ptr ;

                T::register(drv, name, module)
            }),
        }?Error);
        res
    }
}

#[pinned_drop]
impl<T: DriverOps> PinnedDrop for DriverRegistration<T> {
    fn drop(self: Pin<&mut Self>) {
        let drv = unsafe { &mut *self.reg.get() };

        T::unregister(drv);
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
            -> $crate::driver::IdArray<$id_type, U, N>
        where
            $id_type: $crate::driver::RawDeviceId + Copy,
            <$id_type as $crate::driver::RawDeviceId>::RawType: Copy + Clone,
        {
            let mut raw_ids =
                [<$id_type as $crate::driver::RawDeviceId>::ZERO; N];
            let mut i = 0usize;
            while i < N {
                let offset: isize = $crate::driver::IdArray::<$id_type, U, N>::get_offset(i);
                raw_ids[i] = ids[i].to_rawid(offset);
                i += 1;
            }

            // SAFETY: We are passing valid arguments computed with the correct offsets.
            unsafe {
                $crate::driver::IdArray::<$id_type, U, N>::new(raw_ids, infos)
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

/// Counts the number of parenthesis-delimited, comma-separated items.
///
/// # Examples
///
/// ```
/// # use kernel::count_paren_items;
///
/// assert_eq!(0, count_paren_items!());
/// assert_eq!(1, count_paren_items!((A)));
/// assert_eq!(1, count_paren_items!((A),));
/// assert_eq!(2, count_paren_items!((A), (B)));
/// assert_eq!(2, count_paren_items!((A), (B),));
/// assert_eq!(3, count_paren_items!((A), (B), (C)));
/// assert_eq!(3, count_paren_items!((A), (B), (C),));
/// ```
#[macro_export]
macro_rules! count_paren_items {
    (($($item:tt)*), $($remaining:tt)*) => { 1 + $crate::count_paren_items!($($remaining)*) };
    (($($item:tt)*)) => { 1 };
    () => { 0 };
}

/// Converts a comma-separated list of pairs into an array with the first element. That is, it
/// discards the second element of the pair.
///
/// Additionally, it automatically introduces a type if the first element is warpped in curly
/// braces, for example, if it's `{v: 10}`, it becomes `X { v: 10 }`; this is to avoid repeating
/// the type.
///
/// # Examples
///
/// ```
/// # use kernel::first_item;
///
/// #[derive(PartialEq, Debug)]
/// struct X {
///     v: u32,
/// }
///
/// assert_eq!([] as [X; 0], first_item!(X, ));
/// assert_eq!([X { v: 10 }], first_item!(X, ({ v: 10 }, Y)));
/// assert_eq!([X { v: 10 }], first_item!(X, ({ v: 10 }, Y),));
/// assert_eq!([X { v: 10 }], first_item!(X, (X { v: 10 }, Y)));
/// assert_eq!([X { v: 10 }], first_item!(X, (X { v: 10 }, Y),));
/// assert_eq!([X { v: 10 }, X { v: 20 }], first_item!(X, ({ v: 10 }, Y), ({ v: 20 }, Y)));
/// assert_eq!([X { v: 10 }, X { v: 20 }], first_item!(X, ({ v: 10 }, Y), ({ v: 20 }, Y),));
/// assert_eq!([X { v: 10 }, X { v: 20 }], first_item!(X, (X { v: 10 }, Y), (X { v: 20 }, Y)));
/// assert_eq!([X { v: 10 }, X { v: 20 }], first_item!(X, (X { v: 10 }, Y), (X { v: 20 }, Y),));
/// assert_eq!([X { v: 10 }, X { v: 20 }, X { v: 30 }],
///            first_item!(X, ({ v: 10 }, Y), ({ v: 20 }, Y), ({v: 30}, Y)));
/// assert_eq!([X { v: 10 }, X { v: 20 }, X { v: 30 }],
///            first_item!(X, ({ v: 10 }, Y), ({ v: 20 }, Y), ({v: 30}, Y),));
/// assert_eq!([X { v: 10 }, X { v: 20 }, X { v: 30 }],
///            first_item!(X, (X { v: 10 }, Y), (X { v: 20 }, Y), (X {v: 30}, Y)));
/// assert_eq!([X { v: 10 }, X { v: 20 }, X { v: 30 }],
///            first_item!(X, (X { v: 10 }, Y), (X { v: 20 }, Y), (X {v: 30}, Y),));
/// ```
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

/// Converts a comma-separated list of pairs into an array with the second element. That is, it
/// discards the first element of the pair.
///
/// # Examples
///
/// ```
/// # use kernel::second_item;
///
/// assert_eq!([] as [u32; 0], second_item!());
/// assert_eq!([10u32], second_item!((X, 10u32)));
/// assert_eq!([10u32], second_item!((X, 10u32),));
/// assert_eq!([10u32], second_item!(({ X }, 10u32)));
/// assert_eq!([10u32], second_item!(({ X }, 10u32),));
/// assert_eq!([10u32, 20], second_item!((X, 10u32), (X, 20)));
/// assert_eq!([10u32, 20], second_item!((X, 10u32), (X, 20),));
/// assert_eq!([10u32, 20], second_item!(({ X }, 10u32), ({ X }, 20)));
/// assert_eq!([10u32, 20], second_item!(({ X }, 10u32), ({ X }, 20),));
/// assert_eq!([10u32, 20, 30], second_item!((X, 10u32), (X, 20), (X, 30)));
/// assert_eq!([10u32, 20, 30], second_item!((X, 10u32), (X, 20), (X, 30),));
/// assert_eq!([10u32, 20, 30], second_item!(({ X }, 10u32), ({ X }, 20), ({ X }, 30)));
/// assert_eq!([10u32, 20, 30], second_item!(({ X }, 10u32), ({ X }, 20), ({ X }, 30),));
/// ```
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
            $crate::driver::IdArray<$id_type, $data_type, { $crate::count_paren_items!($($t)*) }> =
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
        const $table_name: Option<$crate::driver::IdTable<'static, $id_type, $data_type>> = {
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
