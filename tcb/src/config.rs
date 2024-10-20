use corelib::LinuxResult;
use kernel::error::{Error, KernelResult};

/// 物理页大小
pub const FRAME_SIZE: usize = 0x1000;
/// 物理页大小的位数
pub const FRAME_BITS: usize = 12;

pub fn to_kresult<T>(err: LinuxResult<T>) -> KernelResult<T> {
    match err {
        Ok(v) => Ok(v),
        Err(e) => Err(Error::from_errno(e as i32)),
    }
}
