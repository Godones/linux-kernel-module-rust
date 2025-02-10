use downcast_rs::{impl_downcast, DowncastSync};
use shared_heap::DVec;

use super::LinuxResult;
use crate::Basic;

// #[proxy(EmptyDeviceDomainProxy, SRCU)]
pub trait EmptyDeviceDomain: Basic + DowncastSync {
    fn init(&self) -> LinuxResult<()>;
    fn read(&self, data: DVec<u8>) -> LinuxResult<DVec<u8>>;
    fn write(&self, data: &DVec<u8>) -> LinuxResult<usize>;
}

impl_downcast!(sync EmptyDeviceDomain);
