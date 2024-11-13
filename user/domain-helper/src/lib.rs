mod helper;
use std::error::Error;

use crate::helper::{load_domain, register_domain, unload_domain, update_domain};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
#[repr(u8)]
pub enum DomainTypeRaw {
    EmptyDeviceDomain = 1,
    LogDomain = 2,
    BlockDeviceDomain = 3,
    NvmeBlockDeviceDomain = 4,
}
impl From<u8> for DomainTypeRaw {
    fn from(value: u8) -> Self {
        match value {
            1 => DomainTypeRaw::EmptyDeviceDomain,
            2 => DomainTypeRaw::LogDomain,
            3 => DomainTypeRaw::BlockDeviceDomain,
            4 => DomainTypeRaw::NvmeBlockDeviceDomain,
            _ => panic!("Invalid domain type"),
        }
    }
}

const PATH: &str = "/proc/sys/rust/domain/command";
const DOMAIN_TYPE: &[&str] = &["disk", "init"];

type Result<T> = core::result::Result<T, Box<dyn Error>>;

#[derive(Clone)]
pub struct DomainHelperBuilder {
    ty: Option<DomainTypeRaw>,
    domain_file_name: Option<String>,
    domain_register_ident: Option<String>,
    domain_name: Option<String>,
}

impl DomainHelperBuilder {
    pub fn new() -> Self {
        Self {
            ty: None,
            domain_file_name: None,
            domain_name: None,
            domain_register_ident: None,
        }
    }

    /// Set the domain type
    pub fn ty(mut self, ty: DomainTypeRaw) -> Self {
        self.ty = Some(ty);
        self
    }

    /// Set the domain file name which is used to register the domain
    pub fn domain_file_name(mut self, domain_file_name: &str) -> Self {
        self.domain_file_name = Some(domain_file_name.to_string());
        self
    }

    /// Set the domain name which will be updated
    pub fn domain_name(mut self, domain_name: &str) -> Self {
        self.domain_name = Some(domain_name.to_string());
        self
    }

    /// Set the domain file path which will be opened and registered
    pub fn domain_register_ident(mut self, domain_register_ident: &str) -> Self {
        self.domain_register_ident = Some(domain_register_ident.to_string());
        self
    }
}

impl DomainHelperBuilder {
    pub fn register_domain_file(self) -> Result<()> {
        let ty = self.ty.ok_or("Domain type is not set")?;
        let domain_file_name = self
            .domain_file_name
            .as_ref()
            .ok_or("Domain file name is not set")?;
        let domain_register_ident = self
            .domain_register_ident
            .as_ref()
            .ok_or("Domain file path is not set")?;
        register_domain(domain_file_name, ty as u8, domain_register_ident)?;
        Ok(())
    }
    pub fn update_domain(self) -> Result<()> {
        let ty = self.ty.ok_or("Domain type is not set")?;
        let domain_name = self.domain_name.as_ref().ok_or("Domain name is not set")?;
        let domain_register_ident = self
            .domain_register_ident
            .as_ref()
            .ok_or("Domain file name is not set")?;
        update_domain(domain_name, domain_register_ident, ty as u8)?;
        Ok(())
    }

    pub fn load_domain(self) -> Result<()> {
        let domain_name = self.domain_name.as_ref().ok_or("Domain name is not set")?;
        let ty = self.ty.ok_or("Domain type is not set")?;
        let domain_register_ident = self
            .domain_register_ident
            .as_ref()
            .ok_or("Domain file name is not set")?;
        load_domain(domain_register_ident, domain_name, ty as u8)?;
        Ok(())
    }

    pub fn unload_domain(self) -> Result<()> {
        let domain_name = self.domain_name.as_ref().ok_or("Domain name is not set")?;
        unload_domain(domain_name)?;
        Ok(())
    }
}
