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
pub enum DomainTypeRaw {
    FsDomain = 1,
    BlkDeviceDomain = 2,
    CacheBlkDeviceDomain = 3,
    RtcDomain = 4,
    GpuDomain = 5,
    InputDomain = 6,
    VfsDomain = 7,
    UartDomain = 8,
    PLICDomain = 9,
    TaskDomain = 10,
    SysCallDomain = 11,
    ShadowBlockDomain = 12,
    BufUartDomain = 13,
    NetDeviceDomain = 14,
    BufInputDomain = 15,
    EmptyDeviceDomain = 16,
    DevFsDomain = 17,
    SchedulerDomain = 18,
    LogDomain = 19,
    NetDomain = 20,
}

impl TryFrom<u8> for DomainTypeRaw {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(DomainTypeRaw::FsDomain),
            2 => Ok(DomainTypeRaw::BlkDeviceDomain),
            3 => Ok(DomainTypeRaw::CacheBlkDeviceDomain),
            4 => Ok(DomainTypeRaw::RtcDomain),
            5 => Ok(DomainTypeRaw::GpuDomain),
            6 => Ok(DomainTypeRaw::InputDomain),
            7 => Ok(DomainTypeRaw::VfsDomain),
            8 => Ok(DomainTypeRaw::UartDomain),
            9 => Ok(DomainTypeRaw::PLICDomain),
            10 => Ok(DomainTypeRaw::TaskDomain),
            11 => Ok(DomainTypeRaw::SysCallDomain),
            12 => Ok(DomainTypeRaw::ShadowBlockDomain),
            13 => Ok(DomainTypeRaw::BufUartDomain),
            14 => Ok(DomainTypeRaw::NetDeviceDomain),
            15 => Ok(DomainTypeRaw::BufInputDomain),
            16 => Ok(DomainTypeRaw::EmptyDeviceDomain),
            17 => Ok(DomainTypeRaw::DevFsDomain),
            18 => Ok(DomainTypeRaw::SchedulerDomain),
            19 => Ok(DomainTypeRaw::LogDomain),
            20 => Ok(DomainTypeRaw::NetDomain),
            _ => Err(()),
        }
    }
}
