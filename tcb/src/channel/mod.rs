use alloc::{
    boxed::Box,
    collections::BTreeMap,
    string::{String, ToString},
    vec::Vec,
};

use kernel::sysctl::Sysctl;
use spin::RwLock;

mod command;
pub use command::CommandChannel;
use corelib::{LinuxError, LinuxResult};
use interface::{null_block::BlockArgs, DomainType, DomainTypeRaw};
use kernel::{error::KernelResult, types::Mode};

use crate::{
    create_domain,
    domain_helper::{domain_ref_count, unregister_domain, DOMAIN_SYS},
    domain_proxy::{block_device::BlockDeviceDomainProxy, ProxyBuilder},
    kshim::{BlockDeviceShim, KernelShim},
    register_domain,
};

pub fn init_domain_channel() -> KernelResult<Sysctl<CommandChannel>> {
    println!("Init Domain Channel");
    let command_channel = Sysctl::register(
        c_str!("rust/domain"),
        c_str!("command"),
        CommandChannel::new(),
        Mode::from_int(0o666),
    )?;
    Ok(command_channel)
}

fn register_domain(ident: &str, elf: Vec<u8>, ty: DomainTypeRaw) -> LinuxResult<()> {
    crate::domain_loader::creator::register_domain_elf(ident, elf, ty);
    println!("Register domain: {} ({:?})", ident, ty);
    Ok(())
}

pub fn update_domain(old_ident: &str, new_ident: &str, ty: DomainTypeRaw) -> LinuxResult<()> {
    println!("Update domain: {} -> {} ({:?})", old_ident, new_ident, ty);
    DOMAIN_SYS.sys_update_domain(old_ident, new_ident, ty)?;
    Ok(())
}

static KSHIM_OBJ: RwLock<BTreeMap<String, Box<dyn KernelShim>>> = RwLock::new(BTreeMap::new());

pub fn load_domain(
    register_domain_elf_ident: &str,
    domain_ident: &str,
    ty: DomainTypeRaw,
) -> LinuxResult<()> {
    println!(
        "Load domain: {} ({:?}) -> {} ",
        register_domain_elf_ident, ty, domain_ident
    );
    match ty {
        DomainTypeRaw::BlockDeviceDomain => {
            let (block_device, domain_file_info) = create_domain!(
                BlockDeviceDomainProxy,
                DomainTypeRaw::BlockDeviceDomain,
                register_domain_elf_ident
            )?;
            let args = BlockArgs::default();
            block_device.init_by_box(Box::new(args))?;
            register_domain!(
                domain_ident,
                domain_file_info,
                DomainType::BlockDeviceDomain(block_device.clone()),
                true
            );
            let null_block = BlockDeviceShim::load(block_device).expect("Load block device failed");
            KSHIM_OBJ
                .write()
                .insert(domain_ident.to_string(), Box::new(null_block));
        }
        other => {
            pr_err!("[load_domain] Unsupported domain type: {:?}", other);
            return Err(LinuxError::EINVAL);
        }
    }
    Ok(())
}

pub fn unload_domain(domain_ident: &str) -> LinuxResult<()> {
    println!("Unload domain: {}", domain_ident);
    let ref_count = domain_ref_count(domain_ident);
    if ref_count.is_none() {
        println!("[unload_domain] Domain {} not found", domain_ident);
        return Err(LinuxError::ENOENT);
    }
    let ref_count = ref_count.unwrap();
    if ref_count > 2 {
        println!(
            "[unload_domain] Domain {} is still in use, it has {} references",
            domain_ident, ref_count
        );
        return Err(LinuxError::EBUSY);
    }
    unregister_domain(domain_ident);
    KSHIM_OBJ.write().remove(domain_ident);
    println!("Domain {} unloaded", domain_ident);
    Ok(())
}
