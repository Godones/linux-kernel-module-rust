// SPDX-License-Identifier: GPL-2.0

//! Kernel types.

use alloc::boxed::Box;
use core::{
    cell::UnsafeCell,
    fmt::Debug,
    marker::{PhantomData, PhantomPinned},
    mem::MaybeUninit,
    ops::{Deref, DerefMut},
    pin::Pin,
    ptr::NonNull,
};

use pinned_init::*;

use crate::bindings;

pub struct Mode(bindings::umode_t);

impl Mode {
    pub fn from_int(m: u16) -> Mode {
        Mode(m)
    }

    pub fn as_int(&self) -> u16 {
        self.0
    }
}
/// Used to transfer ownership to and from foreign (non-Rust) languages.
///
/// Ownership is transferred from Rust to a foreign language by calling [`Self::into_foreign`] and
/// later may be transferred back to Rust by calling [`Self::from_foreign`].
///
/// This trait is meant to be used in cases when Rust objects are stored in C objects and
/// eventually "freed" back to Rust.
pub trait ForeignOwnable: Sized {
    /// Type used to immutably borrow a value that is currently foreign-owned.
    type Borrowed<'a>;

    /// Type used to mutably borrow a value that is currently foreign-owned.
    type BorrowedMut<'a>;

    /// Converts a Rust-owned object to a foreign-owned one.
    ///
    /// The foreign representation is a pointer to void.
    fn into_foreign(self) -> *const core::ffi::c_void;

    /// Converts a foreign-owned object back to a Rust-owned one.
    ///
    /// # Safety
    ///
    /// The provided pointer must have been returned by a previous call to [`into_foreign`], and it
    /// must not be passed to `from_foreign` more than once.
    ///
    /// [`into_foreign`]: Self::into_foreign
    unsafe fn from_foreign(ptr: *const core::ffi::c_void) -> Self;

    /// Borrows a foreign-owned object immutably.
    ///
    /// This method provides a way to access a foreign-owned value from Rust immutably. It provides
    /// you with exactly the same abilities as an `&Self` when the value is Rust-owned.
    ///
    /// # Safety
    ///
    /// The provided pointer must have been returned by a previous call to [`into_foreign`], and if
    /// the pointer is ever passed to [`from_foreign`], then that call must happen after the end of
    /// the lifetime 'a.
    ///
    /// [`into_foreign`]: Self::into_foreign
    /// [`from_foreign`]: Self::from_foreign
    unsafe fn borrow<'a>(ptr: *const core::ffi::c_void) -> Self::Borrowed<'a>;

    /// Borrows a foreign-owned object mutably.
    ///
    /// This method provides a way to access a foreign-owned value from Rust mutably. It provides
    /// you with exactly the same abilities as an `&mut Self` when the value is Rust-owned, except
    /// that this method does not let you swap the foreign-owned object for another. (That is, it
    /// does not let you change the address of the void pointer that the foreign code is storing.)
    ///
    /// Note that for types like [`Arc`], an `&mut Arc<T>` only gives you immutable access to the
    /// inner value, so this method also only provides immutable access in that case.
    ///
    /// In the case of `Box<T>`, this method gives you the ability to modify the inner `T`, but it
    /// does not let you change the box itself. That is, you cannot change which allocation the box
    /// points at.
    ///
    /// # Safety
    ///
    /// The provided pointer must have been returned by a previous call to [`into_foreign`], and if
    /// the pointer is ever passed to [`from_foreign`], then that call must happen after the end of
    /// the lifetime 'a.
    ///
    /// The lifetime 'a must not overlap with the lifetime of any other call to [`borrow`] or
    /// `borrow_mut` on the same object.
    ///
    /// [`into_foreign`]: Self::into_foreign
    /// [`from_foreign`]: Self::from_foreign
    /// [`borrow`]: Self::borrow
    /// [`Arc`]: crate::sync::Arc
    unsafe fn borrow_mut<'a>(ptr: *const core::ffi::c_void) -> Self::BorrowedMut<'a>;
}

impl<T: 'static> ForeignOwnable for Box<T> {
    type Borrowed<'a> = &'a T;
    type BorrowedMut<'a> = &'a mut T;

    #[inline(always)]
    fn into_foreign(self) -> *const core::ffi::c_void {
        Box::into_raw(self) as _
    }

    unsafe fn from_foreign(ptr: *const core::ffi::c_void) -> Self {
        // SAFETY: The safety requirements of this function ensure that `ptr` comes from a previous
        // call to `Self::into_foreign`.
        unsafe { Box::from_raw(ptr as _) }
    }

    #[inline(always)]
    unsafe fn borrow<'a>(ptr: *const core::ffi::c_void) -> &'a T {
        // SAFETY: The safety requirements of this method ensure that the object remains alive and
        // immutable for the duration of 'a.
        unsafe { &*ptr.cast() }
    }

    #[inline(always)]
    unsafe fn borrow_mut<'a>(ptr: *const core::ffi::c_void) -> &'a mut T {
        // SAFETY: The safety requirements of this method ensure that the pointer is valid and that
        // nothing else will access the value for the duration of 'a.
        unsafe { &mut *ptr.cast_mut().cast() }
    }
}

impl ForeignOwnable for () {
    type Borrowed<'a> = ();
    type BorrowedMut<'a> = ();

    #[inline(always)]
    fn into_foreign(self) -> *const core::ffi::c_void {
        core::ptr::NonNull::dangling().as_ptr()
    }

    unsafe fn from_foreign(_: *const core::ffi::c_void) -> Self {}

    #[inline(always)]
    unsafe fn borrow<'a>(_: *const core::ffi::c_void) -> Self::Borrowed<'a> {}

    #[inline(always)]
    unsafe fn borrow_mut<'a>(_: *const core::ffi::c_void) -> Self::BorrowedMut<'a> {}
}

impl<T: ForeignOwnable + Deref> ForeignOwnable for Pin<T> {
    type Borrowed<'a> = T::Borrowed<'a>;
    type BorrowedMut<'a> = T::BorrowedMut<'a>;

    fn into_foreign(self) -> *const core::ffi::c_void {
        // SAFETY: We continue to treat the pointer as pinned by returning just a pointer to it to
        // the caller.
        let inner = unsafe { Pin::into_inner_unchecked(self) };
        inner.into_foreign()
    }

    unsafe fn borrow<'a>(ptr: *const core::ffi::c_void) -> Self::Borrowed<'a> {
        // SAFETY: The safety requirements for this function are the same as the ones for
        // `T::borrow`.
        unsafe { T::borrow(ptr) }
    }

    unsafe fn borrow_mut<'a>(ptr: *const core::ffi::c_void) -> Self::BorrowedMut<'a> {
        // SAFETY: The safety requirements for this function are the same as the ones for
        // `T::borrow_mut`.
        unsafe { T::borrow_mut(ptr) }
    }

    unsafe fn from_foreign(p: *const core::ffi::c_void) -> Self {
        // SAFETY: The object was originally pinned.
        // The passed pointer comes from a previous call to `T::into_foreign`.
        unsafe { Pin::new_unchecked(T::from_foreign(p)) }
    }
}

pub struct ScopeGuard<T, F: FnOnce(T)>(Option<(T, F)>);

impl<T, F: FnOnce(T)> ScopeGuard<T, F> {
    /// Creates a new guarded object wrapping the given data and with the given cleanup function.
    pub fn new_with_data(data: T, cleanup_func: F) -> Self {
        // INVARIANT: The struct is being initialised with `Some(_)`.
        Self(Some((data, cleanup_func)))
    }

    /// Prevents the cleanup function from running and returns the guarded data.
    pub fn dismiss(mut self) -> T {
        // INVARIANT: This is the exception case in the invariant; it is not visible to callers
        // because this function consumes `self`.
        self.0.take().unwrap().0
    }
}

impl ScopeGuard<(), fn(())> {
    /// Creates a new guarded object with the given cleanup function.
    pub fn new(cleanup: impl FnOnce()) -> ScopeGuard<(), impl FnOnce(())> {
        ScopeGuard::new_with_data((), move |_| cleanup())
    }
}

impl<T, F: FnOnce(T)> Deref for ScopeGuard<T, F> {
    type Target = T;

    fn deref(&self) -> &T {
        // The type invariants guarantee that `unwrap` will succeed.
        &self.0.as_ref().unwrap().0
    }
}

impl<T, F: FnOnce(T)> DerefMut for ScopeGuard<T, F> {
    fn deref_mut(&mut self) -> &mut T {
        // The type invariants guarantee that `unwrap` will succeed.
        &mut self.0.as_mut().unwrap().0
    }
}

impl<T, F: FnOnce(T)> Drop for ScopeGuard<T, F> {
    fn drop(&mut self) {
        // Run the cleanup function if one is still present.
        if let Some((data, cleanup)) = self.0.take() {
            cleanup(data)
        }
    }
}

/// Stores an opaque value.
///
/// This is meant to be used with FFI objects that are never interpreted by Rust code.
#[repr(transparent)]
pub struct Opaque<T> {
    value: UnsafeCell<MaybeUninit<T>>,
    _pin: PhantomPinned,
}

impl<T> Opaque<T> {
    /// Creates a new opaque value.
    pub const fn new(value: T) -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::new(value)),
            _pin: PhantomPinned,
        }
    }

    /// Creates an uninitialised value.
    pub const fn uninit() -> Self {
        Self {
            value: UnsafeCell::new(MaybeUninit::uninit()),
            _pin: PhantomPinned,
        }
    }

    /// Creates a pin-initializer from the given initializer closure.
    ///
    /// The returned initializer calls the given closure with the pointer to the inner `T` of this
    /// `Opaque`. Since this memory is uninitialized, the closure is not allowed to read from it.
    ///
    /// This function is safe, because the `T` inside of an `Opaque` is allowed to be
    /// uninitialized. Additionally, access to the inner `T` requires `unsafe`, so the caller needs
    /// to verify at that point that the inner value is valid.
    pub fn ffi_init(init_func: impl FnOnce(*mut T)) -> impl PinInit<Self> {
        // SAFETY: We contain a `MaybeUninit`, so it is OK for the `init_func` to not fully
        // initialize the `T`.
        unsafe {
            pin_init_from_closure::<_, ::core::convert::Infallible>(move |slot| {
                init_func(Self::raw_get(slot));
                Ok(())
            })
        }
    }

    /// Similar to [`Self::ffi_init`], except that the closure can fail.
    ///
    /// To avoid leaks on failure, the closure must drop any fields it has initialised before the
    /// failure.
    pub fn try_ffi_init<E>(
        init_func: impl FnOnce(*mut T) -> Result<(), E>,
    ) -> impl PinInit<Self, E> {
        // SAFETY: We contain a `MaybeUninit`, so it is OK for the `init_func` to not fully
        // initialize the `T`.
        unsafe { pin_init_from_closure(|slot| init_func(Self::raw_get(slot))) }
    }

    /// Returns a raw pointer to the opaque data.
    pub fn get(&self) -> *mut T {
        UnsafeCell::get(&self.value).cast::<T>()
    }

    /// Gets the value behind `this`.
    ///
    /// This function is useful to get access to the value without creating intermediate
    /// references.
    pub const fn raw_get(this: *const Self) -> *mut T {
        UnsafeCell::raw_get(this.cast::<UnsafeCell<MaybeUninit<T>>>()).cast::<T>()
    }
}

impl<T: Debug> Debug for Opaque<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Opaque")
            .field("value", unsafe { &*self.get() })
            .finish()
    }
}

/// Types that are _always_ reference counted.
///
/// It allows such types to define their own custom ref increment and decrement functions.
/// Additionally, it allows users to convert from a shared reference `&T` to an owned reference
/// [`ARef<T>`].
///
/// This is usually implemented by wrappers to existing structures on the C side of the code. For
/// Rust code, the recommendation is to use [`Arc`](crate::sync::Arc) to create reference-counted
/// instances of a type.
///
/// # Safety
///
/// Implementers must ensure that increments to the reference count keep the object alive in memory
/// at least until matching decrements are performed.
///
/// Implementers must also ensure that all instances are reference-counted. (Otherwise they
/// won't be able to honour the requirement that [`AlwaysRefCounted::inc_ref`] keep the object
/// alive.)
pub unsafe trait AlwaysRefCounted {
    /// Increments the reference count on the object.
    fn inc_ref(&self);

    /// Decrements the reference count on the object.
    ///
    /// Frees the object when the count reaches zero.
    ///
    /// # Safety
    ///
    /// Callers must ensure that there was a previous matching increment to the reference count,
    /// and that the object is no longer used after its reference count is decremented (as it may
    /// result in the object being freed), unless the caller owns another increment on the refcount
    /// (e.g., it calls [`AlwaysRefCounted::inc_ref`] twice, then calls
    /// [`AlwaysRefCounted::dec_ref`] once).
    unsafe fn dec_ref(obj: NonNull<Self>);
}

/// An owned reference to an always-reference-counted object.
///
/// The object's reference count is automatically decremented when an instance of [`ARef`] is
/// dropped. It is also automatically incremented when a new instance is created via
/// [`ARef::clone`].
///
/// # Invariants
///
/// The pointer stored in `ptr` is non-null and valid for the lifetime of the [`ARef`] instance. In
/// particular, the [`ARef`] instance owns an increment on the underlying object's reference count.
pub struct ARef<T: AlwaysRefCounted> {
    ptr: NonNull<T>,
    _p: PhantomData<T>,
}

// SAFETY: It is safe to send `ARef<T>` to another thread when the underlying `T` is `Sync` because
// it effectively means sharing `&T` (which is safe because `T` is `Sync`); additionally, it needs
// `T` to be `Send` because any thread that has an `ARef<T>` may ultimately access `T` using a
// mutable reference, for example, when the reference count reaches zero and `T` is dropped.
unsafe impl<T: AlwaysRefCounted + Sync + Send> Send for ARef<T> {}

// SAFETY: It is safe to send `&ARef<T>` to another thread when the underlying `T` is `Sync`
// because it effectively means sharing `&T` (which is safe because `T` is `Sync`); additionally,
// it needs `T` to be `Send` because any thread that has a `&ARef<T>` may clone it and get an
// `ARef<T>` on that thread, so the thread may ultimately access `T` using a mutable reference, for
// example, when the reference count reaches zero and `T` is dropped.
unsafe impl<T: AlwaysRefCounted + Sync + Send> Sync for ARef<T> {}

impl<T: AlwaysRefCounted> ARef<T> {
    /// Creates a new instance of [`ARef`].
    ///
    /// It takes over an increment of the reference count on the underlying object.
    ///
    /// # Safety
    ///
    /// Callers must ensure that the reference count was incremented at least once, and that they
    /// are properly relinquishing one increment. That is, if there is only one increment, callers
    /// must not use the underlying object anymore -- it is only safe to do so via the newly
    /// created [`ARef`].
    pub unsafe fn from_raw(ptr: NonNull<T>) -> Self {
        // INVARIANT: The safety requirements guarantee that the new instance now owns the
        // increment on the refcount.
        Self {
            ptr,
            _p: PhantomData,
        }
    }
}

impl<T: AlwaysRefCounted> Clone for ARef<T> {
    fn clone(&self) -> Self {
        self.inc_ref();
        // SAFETY: We just incremented the refcount above.
        unsafe { Self::from_raw(self.ptr) }
    }
}

impl<T: AlwaysRefCounted> Deref for ARef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: The type invariants guarantee that the object is valid.
        unsafe { self.ptr.as_ref() }
    }
}

impl<T: AlwaysRefCounted> From<&T> for ARef<T> {
    fn from(b: &T) -> Self {
        b.inc_ref();
        // SAFETY: We just incremented the refcount above.
        unsafe { Self::from_raw(NonNull::from(b)) }
    }
}

impl<T: AlwaysRefCounted> Drop for ARef<T> {
    fn drop(&mut self) {
        // SAFETY: The type invariants guarantee that the `ARef` owns the reference we're about to
        // decrement.
        unsafe { T::dec_ref(self.ptr) };
    }
}

/// A lockable object.
///
/// Implementers must implement [`Lockable::raw_lock`] and [`Lockable::unlock`], and they'll get a
/// `lock` method that returns a guard that can be used to access the locked object and unlocks it
/// automatically when it goes out of scope.
///
/// `M` is a tag that may be used to speficy which mode to lock/unlock when an object may be
/// locked in multiple ways. For example, inodes have `i_lock` and `i_rwsem` so they can be locked
/// in 3 different modes. If an implementer can be locked in only one way, the default unit type
/// can be omitted for brevity.
///
/// # Safety
///
/// The [`Lockable::raw_lock`] function must indeed lock the object, otherwise we may run into UB
/// if multiple instances believe they have properly serialised access.
pub unsafe trait Lockable<M = ()> {
    /// Locks the object.
    ///
    /// The returned guard will automatically unlock the object when it goes out of scope.
    fn lock(&self) -> Locked<&Self, M> {
        self.raw_lock();

        // SAFETY: The object was locked above, so responsibility to unlock is transferred to the
        // `Locked` instance.
        unsafe { Locked::new(self) }
    }

    /// Locks the object.
    fn raw_lock(&self);

    /// Unlocks the object.
    ///
    /// # Safety
    ///
    /// The object must be locked. And it must not be used again as a locked object before another
    /// call to [`Self::raw_lock`].
    unsafe fn unlock(&self);
}

/// A locked version of an existing type.
///
/// # Invariants
///
/// The object is locked and the responsibility to unlock it belongs to the [`Locked`] instance.
#[repr(transparent)]
pub struct Locked<T: Deref, M = ()>(T, PhantomData<M>)
where
    T::Target: Lockable<M>;

impl<T: Deref, M> Locked<T, M>
where
    T::Target: Lockable<M>,
{
    /// Creates a new instance of [`Locked`].
    ///
    /// # Safety
    ///
    /// The instance of `T` must be locked and the responsibility to unlock it is transferred to
    /// the returned instance of [`Locked`].
    pub unsafe fn new(v: T) -> Self {
        // INVARIANT: The safety requirements ensure that the invariants hold.
        Self(v, PhantomData)
    }
}

impl<T: Deref, M> Deref for Locked<T, M>
where
    T::Target: Lockable<M>,
{
    type Target = T::Target;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Deref, M> Drop for Locked<T, M>
where
    T::Target: Lockable<M>,
{
    fn drop(&mut self) {
        // SAFETY: The type invariants guarantee that the object is locked.
        unsafe { self.0.unlock() }
    }
}

impl<T: Lockable<M> + AlwaysRefCounted, M> From<Locked<&T, M>> for Locked<ARef<T>, M> {
    fn from(value: Locked<&T, M>) -> Self {
        let aref = ARef::<T>::from(value.deref());
        core::mem::forget(value);
        // SAFETY: We forgot the locked value above, so responsibility is transferred to new
        // instance of [`Locked`].
        unsafe { Locked::new(aref) }
    }
}
/// A sum type that always holds either a value of type `L` or `R`.
pub enum Either<L, R> {
    /// Constructs an instance of [`Either`] containing a value of type `L`.
    Left(L),

    /// Constructs an instance of [`Either`] containing a value of type `R`.
    Right(R),
}
/// A type that can be represented in little-endian bytes.
pub trait LittleEndian {
    /// Converts from native to little-endian encoding.
    fn to_le(self) -> Self;

    /// Converts from little-endian to the CPU's encoding.
    fn to_cpu(self) -> Self;
}

macro_rules! define_le {
    ($($t:ty),+) => {
        $(
        impl LittleEndian for $t {
            fn to_le(self) -> Self {
                Self::to_le(self)
            }

            fn to_cpu(self) -> Self {
                Self::from_le(self)
            }
        }
        )*
    };
}

define_le!(u8, u16, u32, u64, i8, i16, i32, i64);

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct LE<T: LittleEndian + Copy>(T);

impl<T: LittleEndian + Copy> LE<T> {
    /// Returns the native-endian value.
    pub fn value(&self) -> T {
        self.0.to_cpu()
    }
}

impl<T: LittleEndian + Copy> core::convert::From<T> for LE<T> {
    fn from(value: T) -> LE<T> {
        LE(value.to_le())
    }
}

/// Specifies that a type is safely readable from byte slices.
///
/// Not all types can be safely read from byte slices; examples from
/// <https://doc.rust-lang.org/reference/behavior-considered-undefined.html> include [`bool`] that
/// must be either `0` or `1`, and [`char`] that cannot be a surrogate or above [`char::MAX`].
///
/// # Safety
///
/// Implementers must ensure that any bit pattern is valid for this type.
pub unsafe trait FromBytes: Sized {
    /// Converts the given byte slice into a shared reference to [`Self`].
    ///
    /// It fails if the size or alignment requirements are not satisfied.
    fn from_bytes(data: &[u8], offset: usize) -> Option<&Self> {
        if offset > data.len() {
            return None;
        }
        let data = &data[offset..];
        let ptr = data.as_ptr();
        if ptr as usize % align_of::<Self>() != 0 || data.len() < size_of::<Self>() {
            return None;
        }
        // SAFETY: The memory is valid for read because we have a reference to it. We have just
        // checked the minimum size and alignment as well.
        Some(unsafe { &*ptr.cast() })
    }

    /// Converts the given byte slice into a shared slice of [`Self`].
    ///
    /// It fails if the size or alignment requirements are not satisfied.
    fn from_bytes_to_slice(data: &[u8]) -> Option<&[Self]> {
        let ptr = data.as_ptr();
        if ptr as usize % align_of::<Self>() != 0 {
            return None;
        }
        // SAFETY: The memory is valid for read because we have a reference to it. We have just
        // checked the minimum alignment as well, and the length of the slice is calculated from
        // the length of `Self`.
        Some(unsafe { core::slice::from_raw_parts(ptr.cast(), data.len() / size_of::<Self>()) })
    }
}

// SAFETY: All bit patterns are acceptable values of the types below.
unsafe impl FromBytes for u8 {}
unsafe impl FromBytes for u16 {}
unsafe impl FromBytes for u32 {}
unsafe impl FromBytes for u64 {}
unsafe impl FromBytes for usize {}
unsafe impl FromBytes for i8 {}
unsafe impl FromBytes for i16 {}
unsafe impl FromBytes for i32 {}
unsafe impl FromBytes for i64 {}
unsafe impl FromBytes for isize {}
unsafe impl<const N: usize, T: FromBytes> FromBytes for [T; N] {}
unsafe impl<T: FromBytes + Copy + LittleEndian> FromBytes for LE<T> {}

#[macro_export]
macro_rules! derive_readable_from_bytes {
    ($($(#[$outer:meta])* $outerv:vis struct $name:ident {
        $($(#[$m:meta])* $v:vis $id:ident : $t:ty),* $(,)?
    })*)=> {
        $(
            $(#[$outer])*
            $outerv struct $name {
                $(
                    $(#[$m])*
                    $v $id: $t,
                )*
            }
            unsafe impl $crate::types::FromBytes for $name {}
            const _: () = {
                const fn is_readable_from_bytes<T: $crate::types::FromBytes>() {}
                $(is_readable_from_bytes::<$t>();)*
            };
        )*
    };
}
