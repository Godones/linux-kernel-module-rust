use alloc::{boxed::Box, vec::Vec};

use kbind::{println, sysctl::Sysctl, Mode};

mod command;
pub use command::CommandChannel;
use corelib::LinuxResult;
use interface::{
    logger::{Level, LogDomain},
    Basic, DomainTypeRaw,
};
use rref::RRefVec;

use crate::{
    create_domain,
    domain_helper::{free_domain_resource, FreeShared},
    domain_proxy::{logger::LogDomainProxy, ProxyBuilder},
};

pub fn init_domain_channel() -> kbind::KernelResult<Sysctl<CommandChannel>> {
    println!("Init Domain Channel");
    let command_channel = Sysctl::register(
        cstr!("rust/domain"),
        cstr!("command"),
        CommandChannel::new(),
        Mode::from_int(0o666),
    )?;
    Ok(command_channel)
}

fn register_domain(ident: &str, elf: Vec<u8>, ty: DomainTypeRaw) -> LinuxResult<()> {
    crate::domain_loader::creator::register_domain_elf(ident, elf, ty);
    if ident == "logger" {
        let (logger, domain_file_info) =
            create_domain!(LogDomainProxy, DomainTypeRaw::LogDomain, "logger")?;
        logger.init_by_box(Box::new(()))?;
        // register_domain!(
        //     "logger",
        //     domain_file_info,
        //     DomainType::LogDomain(logger),
        //     true
        // );
        let id = logger.domain_id();
        let info = RRefVec::from_slice(b"print using logger");
        logger.log(Level::Error, &info)?;
        free_domain_resource(id, FreeShared::Free);
    }
    Ok(())
}
