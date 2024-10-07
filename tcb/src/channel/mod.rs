use alloc::vec::Vec;

use kernel::sysctl::Sysctl;

mod command;
pub use command::CommandChannel;
use corelib::LinuxResult;
use interface::DomainTypeRaw;
use kernel::{error::KernelResult, types::Mode};

use crate::domain_helper::DOMAIN_SYS;

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
    // if ident == "xlogger" {
    //     let (logger, domain_file_info) =
    //         create_domain!(LogDomainProxy, DomainTypeRaw::LogDomain, "xlogger")?;
    //     logger.init_by_box(Box::new(()))?;
    //     // register_domain!(
    //     //     "logger",
    //     //     domain_file_info,
    //     //     DomainType::LogDomain(logger),
    //     //     true
    //     // );
    //     println!("Register logger domain: {:?}", domain_file_info);
    //     let id = logger.domain_id();
    //     let info = RRefVec::from_slice(b"print using logger");
    //     logger.log(Level::Error, &info)?;
    //     free_domain_resource(id, FreeShared::Free);
    // }
    println!("Register domain: {} ({:?})", ident, ty);
    Ok(())
}

pub fn update_domain(old_ident: &str, new_ident: &str, ty: DomainTypeRaw) -> LinuxResult<()> {
    println!("Update domain: {} -> {} ({:?})", old_ident, new_ident, ty);
    DOMAIN_SYS.sys_update_domain(old_ident, new_ident, ty)?;
    Ok(())
}
