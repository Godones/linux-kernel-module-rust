use alloc::boxed::Box;

use kbind::srcu_struct;

use crate::{bindings, bindings::CRcuData, pr_warn};

#[derive(Debug)]
pub struct SRcuData<T> {
    crcu_data: CRcuData,
    ssp: *mut srcu_struct,
    _marker: core::marker::PhantomData<T>,
}
unsafe impl<T> Sync for SRcuData<T> {}
unsafe impl<T> Send for SRcuData<T> {}

impl<T> SRcuData<T> {
    pub fn new(data: T) -> SRcuData<T> {
        let v = Box::into_raw(Box::new(data));
        let ssp = Box::into_raw(Box::new(srcu_struct::default()));
        unsafe {
            bindings::init_srcu_struct(ssp);
        }
        SRcuData {
            crcu_data: CRcuData {
                data_ptr: v as *mut core::ffi::c_void,
            },
            ssp,
            _marker: core::marker::PhantomData,
        }
    }

    pub fn read<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let idx = unsafe { bindings::__srcu_read_lock(self.ssp) };
        let ptr = srcu_defererence::<T>(&self.crcu_data, self.ssp);
        let v = unsafe { &*ptr };
        let r = f(v);
        unsafe {
            bindings::__srcu_read_unlock(self.ssp, idx);
        }
        r
    }

    pub fn read_directly<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        let ptr = srcu_defererence::<T>(&self.crcu_data, self.ssp);
        let v = unsafe { &*ptr };
        f(v)
    }

    pub fn update_directly(&self, data: T) -> Box<T> {
        let old_ptr = self.crcu_data.data_ptr;
        let new_ptr = Box::into_raw(Box::new(data));
        srcu_assign_pointer(&self.crcu_data, new_ptr);
        let old_data = unsafe { Box::from_raw(old_ptr as *mut T) };
        old_data
    }

    pub fn update(&self, data: T) -> Box<T> {
        let old_ptr = self.crcu_data.data_ptr;
        let new_ptr = Box::into_raw(Box::new(data));
        srcu_assign_pointer(&self.crcu_data, new_ptr);
        pr_warn!("before synchronize_srcu");
        synchronize_srcu(self.ssp);
        pr_warn!("after synchronize_srcu");
        let old_data = unsafe { Box::from_raw(old_ptr as *mut T) };
        old_data
    }
}

impl<T> Drop for SRcuData<T> {
    fn drop(&mut self) {
        unsafe {
            bindings::cleanup_srcu_struct(self.ssp);
            let _v = Box::from_raw(self.ssp);
        }
    }
}

fn srcu_defererence<T>(crcu_data: &CRcuData, ssp: *const srcu_struct) -> *const T {
    unsafe {
        let ptr = bindings::srcu_dereference(crcu_data, ssp);
        ptr as *const T
    }
}

fn srcu_assign_pointer<T>(crcu_data: &CRcuData, new_ptr: *const T) {
    unsafe { bindings::rust_helper_rcu_assign_pointer(crcu_data, new_ptr as _) }
}

fn synchronize_srcu(ssp: *const srcu_struct) {
    unsafe { bindings::synchronize_srcu(ssp as *mut srcu_struct) }
}
