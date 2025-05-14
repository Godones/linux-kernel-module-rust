use downcast_rs::{impl_downcast, DowncastSync};
use shared_heap::{DBox, DVec};

use super::LinuxResult;
use crate::Basic;

// #[proxy(EmptyDeviceDomainProxy, SRCU)]
pub trait EmptyDeviceDomain: Basic + DowncastSync {
    fn init(&self) -> LinuxResult<()>;
    fn read(&self, data: DVec<u8>) -> LinuxResult<DVec<u8>>;
    fn write(&self, data: &DVec<u8>) -> LinuxResult<usize>;

    fn no_arg(&self) -> LinuxResult<()>;
    fn one_arg(&self, arg: u64) -> LinuxResult<u64>;
    fn one_darg(&self, arg: DBox<usize>) -> LinuxResult<DBox<usize>>;
    fn two_dargs(&self, arg1: DBox<usize>, arg2: DBox<usize>) -> LinuxResult<(DBox<usize>,DBox<usize>)>;
}

impl_downcast!(sync EmptyDeviceDomain);
