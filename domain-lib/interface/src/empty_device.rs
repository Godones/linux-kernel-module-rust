use downcast_rs::{impl_downcast, DowncastSync};
use rref::RRefVec;

use super::LinuxResult;
use crate::Basic;

// #[proxy(EmptyDeviceDomainProxy, SRCU)]
pub trait EmptyDeviceDomain: Basic + DowncastSync {
    fn init(&self) -> LinuxResult<()>;
    fn read(&self, data: RRefVec<u8>) -> LinuxResult<RRefVec<u8>>;
    fn write(&self, data: &RRefVec<u8>) -> LinuxResult<usize>;
}

impl_downcast!(sync EmptyDeviceDomain);
