#![no_std]
#![feature(trait_upcasting)]
extern crate alloc;

pub mod empty_device;
pub mod logger;
pub mod null_block;
pub mod nvme;

use alloc::sync::Arc;
use core::{any::Any, fmt::Debug};

pub use pconst::LinuxErrno;

use crate::{empty_device::EmptyDeviceDomain, logger::LogDomain, null_block::BlockDeviceDomain};
use crate::nvme::NvmeBlockDeviceDomain;

type LinuxResult<T> = Result<T, LinuxErrno>;

pub trait Basic: Send + Sync + Debug + Any {
    fn domain_id(&self) -> u64;
}

#[derive(Clone, Debug)]
pub enum DomainType {
    EmptyDeviceDomain(Arc<dyn EmptyDeviceDomain>),
    LogDomain(Arc<dyn LogDomain>),
    BlockDeviceDomain(Arc<dyn BlockDeviceDomain>),
    NvmeBlockDeviceDomain(Arc<dyn NvmeBlockDeviceDomain>),
}

impl DomainType {
    pub fn to_raw(&self) -> DomainTypeRaw {
        match self {
            DomainType::EmptyDeviceDomain(_) => DomainTypeRaw::EmptyDeviceDomain,
            DomainType::LogDomain(_) => DomainTypeRaw::LogDomain,
            DomainType::BlockDeviceDomain(_) => DomainTypeRaw::BlockDeviceDomain,
            DomainType::NvmeBlockDeviceDomain(_) => DomainTypeRaw::BlockDeviceDomain,
        }
    }
    pub fn domain_id(&self) -> u64 {
        match self {
            DomainType::EmptyDeviceDomain(d) => d.domain_id(),
            DomainType::LogDomain(d) => d.domain_id(),
            DomainType::BlockDeviceDomain(d) => d.domain_id(),
            DomainType::NvmeBlockDeviceDomain(d) => d.domain_id(),
        }
    }

    pub fn ref_count(&self) -> usize {
        match self {
            DomainType::EmptyDeviceDomain(d) => Arc::strong_count(d),
            DomainType::LogDomain(d) => Arc::strong_count(d),
            DomainType::BlockDeviceDomain(d) => Arc::strong_count(d),
            DomainType::NvmeBlockDeviceDomain(d) => Arc::strong_count(d),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
#[repr(u8)]
pub enum DomainTypeRaw {
    EmptyDeviceDomain = 1,
    LogDomain = 2,
    BlockDeviceDomain = 3,
}

impl TryFrom<u8> for DomainTypeRaw {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DomainTypeRaw::EmptyDeviceDomain),
            2 => Ok(DomainTypeRaw::LogDomain),
            3 => Ok(DomainTypeRaw::BlockDeviceDomain),
            _ => Err(()),
        }
    }
}
