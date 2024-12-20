use alloc::{boxed::Box, vec};
use core::{mem, ptr, sync::atomic};

use crate::{
    bindings,
    buf::{KernelSlicePtr, KernelSlicePtrWriter},
    error, println,
    str::CStr,
    types,
};

pub trait SysctlStorage: Sync {
    fn store_value(&self, data: &[u8]) -> (usize, error::KernelResult<()>);
    fn read_value(&self, data: &mut KernelSlicePtrWriter) -> (usize, error::KernelResult<()>);
}

fn trim_whitespace(mut data: &[u8]) -> &[u8] {
    while !data.is_empty() && (data[0] == b' ' || data[0] == b'\t' || data[0] == b'\n') {
        data = &data[1..];
    }
    while !data.is_empty()
        && (data[data.len() - 1] == b' '
            || data[data.len() - 1] == b'\t'
            || data[data.len() - 1] == b'\n')
    {
        data = &data[..data.len() - 1];
    }
    data
}

impl<T> SysctlStorage for &T
where
    T: SysctlStorage,
{
    fn store_value(&self, data: &[u8]) -> (usize, error::KernelResult<()>) {
        (*self).store_value(data)
    }

    fn read_value(&self, data: &mut KernelSlicePtrWriter) -> (usize, error::KernelResult<()>) {
        (*self).read_value(data)
    }
}

impl SysctlStorage for atomic::AtomicBool {
    fn store_value(&self, data: &[u8]) -> (usize, error::KernelResult<()>) {
        let result = match trim_whitespace(data) {
            b"0" => {
                self.store(false, atomic::Ordering::Relaxed);
                Ok(())
            }
            b"1" => {
                self.store(true, atomic::Ordering::Relaxed);
                Ok(())
            }
            _ => Err(error::linux_err::EINVAL),
        };
        (data.len(), result)
    }

    fn read_value(&self, data: &mut KernelSlicePtrWriter) -> (usize, error::KernelResult<()>) {
        let value = if self.load(atomic::Ordering::Relaxed) {
            b"1\n"
        } else {
            b"0\n"
        };
        (value.len(), data.write(value))
    }
}

pub struct Sysctl<T: SysctlStorage> {
    inner: Box<T>,
    // Responsible for keeping the ctl_table alive.
    _table: Box<[bindings::ctl_table]>,
    header: *mut bindings::ctl_table_header,
}

// This is safe because the only public method we have is get(), which returns
// &T, and T: Sync. Any new methods must adhere to this requirement.
unsafe impl<T: SysctlStorage> Sync for Sysctl<T> {}

unsafe extern "C" fn proc_handler<T: SysctlStorage>(
    ctl: *mut bindings::ctl_table,
    write: core::ffi::c_int,
    buffer: *mut core::ffi::c_void,
    len: *mut usize,
    ppos: *mut bindings::loff_t,
) -> core::ffi::c_int {
    // println!(
    //     "proc_handler: ctl={:p}, write={}, buffer={:p}, len={}, ppos={}",
    //     ctl, write, buffer, *len, *ppos
    // );
    // If we're reading from some offset other than the beginning of the file,
    // return an empty read to signal EOF.
    if *ppos != 0 && write == 0 {
        *len = 0;
        return 0;
    }

    let data = match KernelSlicePtr::new(buffer, *len) {
        Ok(ptr) => ptr,
        Err(e) => {
            println!("proc_handler: UserSlicePtr::new failed: {:?}", e);
            return e.to_errno();
        }
    };
    let storage = &*((*ctl).data as *const T);
    let (bytes_processed, result) = if write != 0 {
        let data = match data.read_all() {
            Ok(r) => r,
            Err(e) => return e.to_errno(),
        };
        storage.store_value(&data)
    } else {
        let mut writer = data.writer();
        storage.read_value(&mut writer)
    };
    *len = bytes_processed;
    *ppos += *len as bindings::loff_t;
    match result {
        Ok(()) => 0,
        Err(e) => e.to_errno(),
    }
}

impl<T: SysctlStorage> Sysctl<T> {
    pub fn register(
        path: &'static CStr,
        name: &'static CStr,
        storage: T,
        mode: types::Mode,
    ) -> error::KernelResult<Sysctl<T>> {
        if name.contains(&b'/') {
            return Err(error::linux_err::EINVAL);
        }

        let storage = Box::new(storage);
        let mut table = vec![
            bindings::ctl_table {
                procname: name.as_ptr() as *const i8,
                mode: mode.as_int(),
                data: &*storage as *const T as *mut core::ffi::c_void,
                proc_handler: Some(proc_handler::<T>),
                maxlen: 0,
                #[cfg(all(not(v6_8), not(v6_6)))]
                child: ptr::null_mut(),
                poll: ptr::null_mut(),
                extra1: ptr::null_mut(),
                extra2: ptr::null_mut(),
                #[cfg(any(v6_6, v6_8))]
                type_: bindings::ctl_table_SYSCTL_TABLE_TYPE_DEFAULT,
            },
            unsafe { mem::zeroed() },
        ]
        .into_boxed_slice();

        let result = unsafe {
            #[cfg(any(v6_6, v6_8))]
            {
                bindings::register_sysctl_sz(
                    path.as_ptr() as *const i8,
                    table.as_mut_ptr(),
                    table.len(),
                )
            }
            #[cfg(all(not(v6_8), not(v6_6)))]
            bindings::register_sysctl(path.as_ptr() as *const i8, table.as_mut_ptr())
        };
        if result.is_null() {
            return Err(error::linux_err::ENOMEM);
        }

        Ok(Sysctl {
            inner: storage,
            _table: table,
            header: result,
        })
    }

    pub fn get(&self) -> &T {
        &self.inner
    }
}

impl<T: SysctlStorage> Drop for Sysctl<T> {
    fn drop(&mut self) {
        unsafe {
            bindings::unregister_sysctl_table(self.header);
        }
        self.header = ptr::null_mut();
    }
}
