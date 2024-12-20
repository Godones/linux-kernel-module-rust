mod resource;
mod sheap;
mod storage_heap;
mod syscall;

extern crate alloc;

use alloc::{
    boxed::Box,
    collections::BTreeMap,
    format,
    string::{String, ToString},
    sync::Arc,
};
use core::sync::atomic::AtomicU64;

use basic::DomainInfoSet;
use corelib::{
    domain_info::{DomainDataInfo, DomainFileInfo, DomainInfo},
    LinuxResult,
};
pub use interface::DomainType;
use ksync::{Lazy, Mutex, Once};
pub use resource::*;
pub use sheap::{checkout_shared_data, FreeShared, SHARED_HEAP_ALLOCATOR};
pub use storage_heap::*;
pub use syscall::DOMAIN_SYS;

static DOMAIN_IDS: AtomicU64 = AtomicU64::new(0);

struct DomainContainer {
    domains: BTreeMap<String, DomainType>,
    ty_counter: BTreeMap<String, u64>,
}

unsafe impl Send for DomainContainer {}

impl DomainContainer {
    pub const fn new() -> Self {
        Self {
            domains: BTreeMap::new(),
            ty_counter: BTreeMap::new(),
        }
    }
}
impl DomainContainer {
    fn insert(&mut self, identifier: String, domain: DomainType, unique: bool) -> String {
        if unique {
            if self.domains.contains_key(&identifier) {
                panic!(
                    "domain {} already exists, but it should be unique",
                    identifier
                );
            }
            println!(
                "<attach domain>: {}, it's name is {}",
                identifier, identifier
            );
            self.domains.insert(identifier.clone(), domain);
            identifier
        } else {
            let counter = self.ty_counter.entry(identifier.clone()).or_insert(0);
            *counter += 1;
            let name = format!("{}-{}", identifier, counter);
            println!("<attach domain>: {}, it's name is {}", identifier, name);
            self.domains.insert(name.clone(), domain);
            name
        }
    }
    fn get(&self, name: &str) -> Option<DomainType> {
        self.domains.get(name).cloned()
    }

    fn ref_count(&self, name: &str) -> Option<usize> {
        self.domains.get(name).map(|domain| domain.ref_count())
    }
}

static DOMAIN_CONTAINER: Mutex<DomainContainer> = Mutex::new(DomainContainer::new());
static DOMAIN_CREATE: Once<Box<dyn DomainCreate>> = Once::new();
pub static DOMAIN_INFO: Lazy<Arc<DomainInfoSet>> =
    Lazy::new(|| Arc::new(DomainInfoSet::new(DomainInfo::new())));
/// Allocate a domain id
pub fn alloc_domain_id() -> u64 {
    DOMAIN_IDS.fetch_add(1, core::sync::atomic::Ordering::SeqCst)
}

/// Initialize the domain creation function
pub fn init_domain_create(domain_create: Box<dyn DomainCreate>) {
    DOMAIN_CREATE.call_once(|| domain_create);
}

/// find the domain which name is `domain_identifier`
pub fn query_domain(domain_identifier: &str) -> Option<DomainType> {
    DOMAIN_CONTAINER.lock().get(domain_identifier)
}

/// Register a domain with a  identifier which may be unique
pub fn register_domain(
    identifier: &str,
    domain_file: DomainFileInfo,
    domain: DomainType,
    unique: bool,
) -> String {
    let domain_id = domain.domain_id();
    let ty = domain.to_raw();
    let res = DOMAIN_CONTAINER
        .lock()
        .insert(identifier.to_string(), domain, unique);
    let domain_data = DomainDataInfo {
        name: res.clone(),
        ty,
        panic_count: 0,
        file_info: domain_file,
    };

    DOMAIN_INFO
        .lock()
        .domain_list
        .insert(domain_id, domain_data);
    res
}

/// Unregister the domain with the given identifier
///
/// User should make sure the domain is not used by any other
pub fn unregister_domain(identifier: &str) {
    let domain = DOMAIN_CONTAINER.lock().domains.remove(identifier);
    if let Some(domain) = domain {
        let domain_id = domain.domain_id();
        DOMAIN_INFO.lock().domain_list.remove(&domain_id);
    }
}

/// Get the reference count of the domain
pub fn domain_ref_count(identifier: &str) -> Option<usize> {
    let container = DOMAIN_CONTAINER.lock();
    container.ref_count(identifier)
}

/// Register the domain elf data with the given identifier.
///
/// # Arguments
/// `ident` - The identifier of the domain.
///
/// `domain_file` - The domain file information.
///
/// `domain` - The domain type
///
#[macro_export]
macro_rules! register_domain {
    ($ident:expr,$domain_file:expr,$domain:expr,$unique:expr) => {
        $crate::domain_helper::register_domain($ident, $domain_file, $domain, $unique)
    };
}

pub trait DomainCreate: Send + Sync {
    fn create_domain(
        &self,
        domain_file_name: &str,
        identifier: &mut [u8],
    ) -> LinuxResult<DomainType>;
}
