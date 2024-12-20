use alloc::boxed::Box;

use crate::{bindings, bindings::CRcuData, pr_warn};

#[derive(Debug)]
pub struct RcuData<T> {
    crcu_data: CRcuData,
    _marker: core::marker::PhantomData<T>,
}

unsafe impl<T> Sync for RcuData<T> {}
unsafe impl<T> Send for RcuData<T> {}

impl<T> RcuData<T> {
    pub fn new(data: T) -> RcuData<T> {
        let v = Box::into_raw(Box::new(data));
        RcuData {
            crcu_data: CRcuData {
                data_ptr: v as *mut core::ffi::c_void,
            },
            _marker: core::marker::PhantomData,
        }
    }
    /// Read the data
    ///
    /// This `f` must be called between [rcu_read_lock] and [rcu_read_unlock]
    pub fn read<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        rcu_read_lock();
        let ptr = rcu_defererence::<T>(&self.crcu_data);
        let v = unsafe { &*ptr };
        let r = f(v);
        rcu_read_unlock();
        r
    }

    /// Update the rcu data
    ///
    /// This primitive protects concurrent readers from the updater, not concurrent updates from each other!
    /// You therefore still need to use locking (or something similar) to keep concurrent updates from interfering
    /// with each other.
    pub fn update(&self, data: T) -> Box<T> {
        let old_ptr = self.crcu_data.data_ptr;
        let new_ptr = Box::into_raw(Box::new(data));
        rcu_assign_pointer(&self.crcu_data, new_ptr);
        pr_warn!("before synchronize_rcu");
        synchronize_rcu();
        pr_warn!("after synchronize_rcu");
        let old_data = unsafe { Box::from_raw(old_ptr as *mut T) };
        old_data
    }
}

fn rcu_read_lock() {
    unsafe { bindings::rust_helper_rcu_read_lock() }
}

fn rcu_read_unlock() {
    unsafe { bindings::rust_helper_rcu_read_unlock() }
}

fn synchronize_rcu() {
    unsafe { bindings::rust_helper_synchronize_rcu() }
}

fn rcu_defererence<T>(crcu_data: &CRcuData) -> *const T {
    unsafe {
        let ptr = bindings::rust_helper_rcu_dereference(crcu_data);
        ptr as *const T
    }
}

fn rcu_assign_pointer<T>(crcu_data: &CRcuData, new_ptr: *const T) {
    unsafe { bindings::rust_helper_rcu_assign_pointer(crcu_data, new_ptr as _) }
}
