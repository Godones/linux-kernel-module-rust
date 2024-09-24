#![no_std]
#![feature(trait_upcasting)]
extern crate alloc;

pub mod empty_device;
pub mod logger;

use alloc::sync::Arc;
use core::{any::Any, fmt::Debug};

pub use pconst::LinuxErrno;

use crate::{empty_device::EmptyDeviceDomain, logger::LogDomain};

type LinuxResult<T> = Result<T, LinuxErrno>;

pub trait Basic: Send + Sync + Debug + Any {
    fn domain_id(&self) -> u64;
}

#[derive(Clone, Debug)]
pub enum DomainType {
    EmptyDeviceDomain(Arc<dyn EmptyDeviceDomain>),
    LogDomain(Arc<dyn LogDomain>),
}

impl DomainType {
    pub fn to_raw(&self) -> DomainTypeRaw {
        match self {
            DomainType::EmptyDeviceDomain(_) => DomainTypeRaw::EmptyDeviceDomain,
            DomainType::LogDomain(_) => DomainTypeRaw::LogDomain,
        }
    }
    pub fn domain_id(&self) -> u64 {
        match self {
            DomainType::EmptyDeviceDomain(d) => d.domain_id(),
            DomainType::LogDomain(d) => d.domain_id(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
#[repr(u8)]
pub enum DomainTypeRaw {
    EmptyDeviceDomain = 1,
    LogDomain = 2,
}

impl TryFrom<u8> for DomainTypeRaw {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DomainTypeRaw::EmptyDeviceDomain),
            2 => Ok(DomainTypeRaw::LogDomain),
            _ => Err(()),
        }
    }
}
