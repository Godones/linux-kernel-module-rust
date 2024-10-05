use alloc::boxed::Box;
use core::{default::Default, marker, ptr::addr_of_mut};

use crate::{bindings, error, types::CStr};

pub struct Registration<T: FileSystem> {
    _phantom: marker::PhantomData<T>,
    ptr: Box<bindings::file_system_type>,
}

// This is safe because Registration doesn't actually expose any methods.
unsafe impl<T> Sync for Registration<T> where T: FileSystem {}

impl<T: FileSystem> Drop for Registration<T> {
    fn drop(&mut self) {
        unsafe { bindings::unregister_filesystem(&mut *self.ptr) };
    }
}

pub trait FileSystem: Sync {
    const NAME: CStr<'static>;
    const FLAGS: FileSystemFlags;
}

bitflags::bitflags! {
    pub struct FileSystemFlags: core::ffi::c_int {
        const REQUIRES_DEV = bindings::FS_REQUIRES_DEV as core::ffi::c_int;
        const BINARY_MOUNTDATA = bindings::FS_BINARY_MOUNTDATA as core::ffi::c_int;
        const HAS_SUBTYPE = bindings::FS_HAS_SUBTYPE as core::ffi::c_int;
        const USERNS_MOUNT = bindings::FS_USERNS_MOUNT as core::ffi::c_int;
        const RENAME_DOES_D_MOVE = bindings::FS_RENAME_DOES_D_MOVE as core::ffi::c_int;
    }
}

extern "C" fn fill_super_callback<T: FileSystem>(
    _sb: *mut bindings::super_block,
    _data: *mut core::ffi::c_void,
    _silent: core::ffi::c_int,
) -> core::ffi::c_int {
    // T::fill_super(...)
    // This should actually create an object that gets dropped by
    // file_system_registration::kill_sb. You can point to it with
    // sb->s_fs_info.
    unimplemented!();
}

extern "C" fn mount_callback<T: FileSystem>(
    fs_type: *mut bindings::file_system_type,
    flags: core::ffi::c_int,
    _dev_name: *const core::ffi::c_char,
    data: *mut core::ffi::c_void,
) -> *mut bindings::dentry {
    unsafe { bindings::mount_nodev(fs_type, flags, data, Some(fill_super_callback::<T>)) }
}

pub fn register<T: FileSystem>() -> error::KernelResult<Registration<T>> {
    let mut fs_registration = Registration {
        ptr: Box::new(bindings::file_system_type {
            name: T::NAME.as_ptr() as *const i8,
            owner: addr_of_mut!(bindings::__this_module),
            fs_flags: T::FLAGS.bits(),
            mount: Some(mount_callback::<T>),
            kill_sb: Some(bindings::kill_litter_super),

            ..Default::default()
        }),
        _phantom: marker::PhantomData,
    };
    let result = unsafe { bindings::register_filesystem(&mut *fs_registration.ptr) };
    if result != 0 {
        return Err(error::Error::from_errno(result));
    }

    Ok(fs_registration)
}
