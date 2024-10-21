use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use core::sync::atomic;

use command::{Command, Response};
use interface::DomainTypeRaw;
use kernel::{
    buf::KernelSlicePtrWriter,
    error::{linux_err, KernelResult},
    pr_err, println,
    sysctl::SysctlStorage,
};
use spin::Mutex;

use crate::channel::update_domain;

#[derive(Debug)]
pub struct CommandChannel {
    id: atomic::AtomicU64,
    inner: Mutex<CommandChannelInner>,
}
#[derive(Debug)]
struct CommandChannelInner {
    id: Option<u64>,
    domain_type: Option<DomainTypeRaw>,
    domain_ident: Option<String>,
    domain_size: Option<usize>,
    domain_data: Option<Vec<u8>>,
    response: Option<Response>,
}

impl CommandChannel {
    pub fn new() -> Self {
        Self {
            id: atomic::AtomicU64::new(0),
            inner: Mutex::new(CommandChannelInner {
                id: None,
                domain_type: None,
                domain_ident: None,
                domain_size: None,
                domain_data: None,
                response: None,
            }),
        }
    }
}

impl SysctlStorage for CommandChannel {
    fn store_value(&self, data: &[u8]) -> (usize, KernelResult<()>) {
        let command = Command::parse(data);
        let mut inner = self.inner.lock();
        match command {
            Some(Command::Start(ref start_command)) => {
                println!("Command: {:?}", command);
                inner.id = Some(self.id.fetch_add(1, atomic::Ordering::Relaxed));
                let ty = DomainTypeRaw::try_from(start_command.domain_type);
                if ty.is_err() {
                    pr_err!("Invalid domain type");
                    return (0, Err(linux_err::EINVAL));
                }
                let ty = ty.unwrap();
                inner.domain_type = Some(ty);
                inner.domain_ident = Some(start_command.register_domain_elf_ident.to_string());
                inner.domain_size = Some(start_command.domain_size);
                inner.domain_data = Some(Vec::with_capacity(start_command.domain_size));
                // set res
                inner.response = Some(Response::Ok(inner.id.unwrap() as usize));
                (data.len(), Ok(()))
            }
            Some(Command::Send(send_command)) => {
                if send_command.id != inner.id.unwrap() {
                    pr_err!("Invalid id");
                    return (0, Err(linux_err::EINVAL));
                }
                if send_command.bytes != send_command.data.len() {
                    pr_err!("Invalid data length");
                    return (0, Err(linux_err::EINVAL));
                }
                inner
                    .domain_data
                    .as_mut()
                    .unwrap()
                    .extend_from_slice(send_command.data);
                // set res
                inner.response = Some(Response::Receive(
                    inner.id.unwrap() as usize,
                    send_command.data_id,
                    send_command.bytes,
                ));
                (data.len(), Ok(()))
            }
            Some(Command::Stop(ref stop_command)) => {
                println!("Command: {:?}", command);
                if stop_command.id != inner.id.unwrap() {
                    pr_err!("Invalid id");
                    return (0, Err(linux_err::EINVAL));
                }
                let id = inner.id.take().unwrap();
                let domain_elf = inner.domain_data.take().unwrap();
                let ty = inner.domain_type.take().unwrap();
                let ident = inner.domain_ident.take().unwrap();
                inner.domain_size = None;

                super::register_domain(ident.as_str(), domain_elf, ty).unwrap();

                // set res
                inner.response = Some(Response::Ok(id as usize));
                (data.len(), Ok(()))
            }
            Some(Command::Update(ref update_command)) => {
                println!("Command: {:?}", command);
                let old_domain_ident = update_command.domain_ident;
                let new_domain_ident = update_command.register_domain_elf_ident;
                let domain_type = DomainTypeRaw::try_from(update_command.domain_type);
                if domain_type.is_err() {
                    pr_err!("Invalid domain type");
                    return (0, Err(linux_err::EINVAL));
                }
                let domain_type = domain_type.unwrap();
                update_domain(old_domain_ident, new_domain_ident, domain_type).unwrap();
                inner.response = Some(Response::Ok(0));
                (data.len(), Ok(()))
            }
            Some(Command::Load(ref load_command)) => {
                println!("Command: {:?}", command);
                let ty = DomainTypeRaw::try_from(load_command.domain_type);
                if ty.is_err() {
                    pr_err!("Invalid domain type");
                    return (0, Err(linux_err::EINVAL));
                }
                let ty = ty.unwrap();
                let res = super::load_domain(
                    load_command.register_domain_elf_ident,
                    load_command.domain_ident,
                    ty,
                );
                if res.is_err() {
                    return (0, Err(linux_err::EINVAL));
                }
                inner.response = Some(Response::Ok(0));
                (data.len(), Ok(()))
            }
            Some(Command::Unload(ref unload_command)) => {
                println!("Command: {:?}", command);
                let res = super::unload_domain(unload_command.domain_ident);
                if res.is_err() {
                    return (0, Err(linux_err::EINVAL));
                }
                inner.response = Some(Response::Ok(0));
                (data.len(), Ok(()))
            }
            None => {
                pr_err!("Invalid command format");
                (0, Err(linux_err::EINVAL))
            }
            Some(c) => {
                pr_err!("Invalid command: {:?}", c);
                (0, Err(linux_err::EINVAL))
            }
        }
    }

    fn read_value(&self, data: &mut KernelSlicePtrWriter) -> (usize, KernelResult<()>) {
        let mut inner = self.inner.lock();
        println!("Response: {:?}", inner.response);
        if inner.response.is_none() {
            return (0, Err(linux_err::EAGAIN));
        }
        let res = inner.response.as_ref().unwrap().to_bytes();
        if data.len() < res.len() {
            return (0, Err(linux_err::EAGAIN));
        }
        inner.response = None;
        (res.len(), data.write(&res))
    }
}
