/// A safe wrapper around a raw pointer.
#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct SafePtr(*mut core::ffi::c_void);

impl SafePtr {
    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid.
    pub unsafe fn new<T>(ptr: *mut T) -> Self {
        Self(ptr as _)
    }

    /// # Safety
    ///
    /// The caller must ensure that the pointer is valid.
    pub unsafe fn raw_ptr<T>(&self) -> *mut T {
        self.0 as _
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

unsafe impl Send for SafePtr {}
unsafe impl Sync for SafePtr {}
