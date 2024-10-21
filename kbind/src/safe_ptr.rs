/// A safe wrapper around a raw pointer.
#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct SafePtr(*mut core::ffi::c_void);

impl SafePtr {
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid.
    pub unsafe fn new(ptr: *mut core::ffi::c_void) -> Self {
        Self(ptr)
    }

    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid.
    pub unsafe fn raw_ptr(&self) -> *mut core::ffi::c_void {
        self.0
    }

    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid.
    pub unsafe fn as_ref<T>(&self) -> &T {
        &*(self.0 as *const T)
    }

    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid.
    pub unsafe fn as_mut<T>(&self) -> &mut T {
        &mut *(self.0 as *mut T)
    }
}
