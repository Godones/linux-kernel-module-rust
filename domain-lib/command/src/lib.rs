#![no_std]
extern crate alloc;

use alloc::{format, vec::Vec};

#[derive(Debug)]
pub enum Command<'a> {
    Start(StartCommand<'a>),
    Send(SendCommand<'a>),
    Stop(StopCommand),
    Exit(ExitCommand),
    Update(UpdateCommand<'a>),
    Load(LoadCommand<'a>),
    Unload(UnloadCommand<'a>),
}

/// Command to update domain
#[derive(Debug)]
pub struct UpdateCommand<'a> {
    pub domain_ident: &'a str,
    pub register_domain_elf_ident: &'a str,
    pub domain_type: u8,
}

/// Command to start to send domain data
#[derive(Debug)]
pub struct StartCommand<'a> {
    pub register_domain_elf_ident: &'a str,
    pub domain_type: u8,
    pub domain_size: usize,
}

/// Command to Load domain
#[derive(Debug)]
pub struct LoadCommand<'a> {
    pub register_domain_elf_ident: &'a str,
    pub domain_ident: &'a str,
    pub domain_type: u8,
}
#[derive(Debug)]
/// Command to Unload domain
pub struct UnloadCommand<'a> {
    pub domain_ident: &'a str,
}

/// Command to send domain data
#[derive(Debug)]
pub struct SendCommand<'a> {
    pub id: u64,
    pub data_id: usize,
    pub bytes: usize,
    pub data: &'a [u8],
}
/// Command to stop sending domain data
#[derive(Debug)]
pub struct StopCommand {
    pub id: u64,
}

#[derive(Debug)]
pub struct ExitCommand {
    pub id: u64,
}

impl Command<'_> {
    pub fn parse(data: &[u8]) -> Option<Command> {
        let mut iter = data.splitn(2, |&x| x == b':');
        let command = iter.next()?;
        let command = core::str::from_utf8(command).ok()?;
        match command {
            "start" => {
                let start_command = Self::parse_start(iter.next()?)?;
                Some(Command::Start(start_command))
            }
            "send" => {
                let send_command = Self::parse_send(iter.next()?)?;
                Some(Command::Send(send_command))
            }
            "stop" => {
                let stop_command = Self::parse_stop(iter.next()?)?;
                Some(Command::Stop(stop_command))
            }
            "update" => {
                let update_command = Self::parse_update(iter.next()?)?;
                Some(Command::Update(update_command))
            }
            "load" => {
                let load_command = Self::parse_load(iter.next()?)?;
                Some(Command::Load(load_command))
            }
            "unload" => {
                let unload_command = Self::parse_unload(iter.next()?)?;
                Some(Command::Unload(unload_command))
            }
            _ => None,
        }
    }

    fn parse_load(data: &[u8]) -> Option<LoadCommand> {
        let mut iter = data.splitn(3, |&x| x == b':');
        let register_domain_elf_ident = iter.next()?;
        let register_domain_elf_ident = core::str::from_utf8(register_domain_elf_ident).ok()?;

        let domain_ident = iter.next()?;
        let domain_ident = core::str::from_utf8(domain_ident).ok()?;

        let domain_type = iter.next()?;
        let domain_type = core::str::from_utf8(domain_type).ok()?;
        let domain_type_num = domain_type.parse::<u8>().ok()?;
        let domain_type = domain_type_num;
        Some(LoadCommand {
            register_domain_elf_ident,
            domain_ident,
            domain_type,
        })
    }

    fn parse_unload(data: &[u8]) -> Option<UnloadCommand> {
        let domain_ident = core::str::from_utf8(data).ok()?;
        Some(UnloadCommand { domain_ident })
    }

    fn parse_start(data: &[u8]) -> Option<StartCommand> {
        let mut iter = data.splitn(3, |&x| x == b':');
        let domain_ident = iter.next()?;
        let domain_ident = core::str::from_utf8(domain_ident).ok()?;
        let domain_type = iter.next()?;
        let domain_type = core::str::from_utf8(domain_type).ok()?;
        let domain_type_num = domain_type.parse::<u8>().ok()?;
        let domain_type = domain_type_num;
        let domain_size = iter.next()?;
        let domain_size = core::str::from_utf8(domain_size).ok()?;
        let domain_size = domain_size.parse::<usize>().ok()?;
        Some(StartCommand {
            register_domain_elf_ident: domain_ident,
            domain_type,
            domain_size,
        })
    }

    fn parse_send(data: &[u8]) -> Option<SendCommand> {
        let mut iter = data.splitn(4, |&x| x == b':');
        let id = iter.next()?;
        let id = core::str::from_utf8(id).ok()?;
        let id = id.parse::<u64>().ok()?;
        let data_id = iter.next()?;
        let data_id = core::str::from_utf8(data_id).ok()?;
        let data_id = data_id.parse::<usize>().ok()?;
        let bytes = iter.next()?;
        let bytes = core::str::from_utf8(bytes).ok()?;
        let bytes = bytes.parse::<usize>().ok()?;
        let data = iter.next()?;
        Some(SendCommand {
            id,
            data_id,
            bytes,
            data,
        })
    }

    fn parse_stop(data: &[u8]) -> Option<StopCommand> {
        let id = core::str::from_utf8(data).ok()?;
        let id = id.parse::<u64>().ok()?;
        Some(StopCommand { id })
    }

    fn parse_update(data: &[u8]) -> Option<UpdateCommand> {
        let mut iter = data.splitn(3, |&x| x == b':');
        let old_domain_ident = iter.next()?;
        let old_domain_ident = core::str::from_utf8(old_domain_ident).ok()?;
        let new_domain_ident = iter.next()?;
        let new_domain_ident = core::str::from_utf8(new_domain_ident).ok()?;
        let domain_type = iter.next()?;
        let domain_type = core::str::from_utf8(domain_type).ok()?;
        let domain_type_num = domain_type.parse::<u8>().ok()?;
        let domain_type = domain_type_num;
        Some(UpdateCommand {
            domain_ident: old_domain_ident,
            register_domain_elf_ident: new_domain_ident,
            domain_type,
        })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Command::Start(start_command) => format!(
                "start:{}:{}:{}",
                start_command.register_domain_elf_ident,
                start_command.domain_type,
                start_command.domain_size
            )
            .as_bytes()
            .to_vec(),
            Command::Send(send_command) => {
                let mut head = format!(
                    "send:{}:{}:{}:",
                    send_command.id, send_command.data_id, send_command.bytes
                )
                .as_bytes()
                .to_vec();
                head.extend_from_slice(send_command.data);
                head
            }
            Command::Load(load_command) => format!(
                "load:{}:{}:{}",
                load_command.register_domain_elf_ident,
                load_command.domain_ident,
                load_command.domain_type
            )
            .as_bytes()
            .to_vec(),
            Command::Unload(unload_command) => format!("unload:{}", unload_command.domain_ident)
                .as_bytes()
                .to_vec(),
            Command::Stop(stop_command) => format!("stop:{}", stop_command.id).as_bytes().to_vec(),
            Command::Exit(exit_command) => format!("exit:{}", exit_command.id).as_bytes().to_vec(),
            Command::Update(update_command) => format!(
                "update:{}:{}:{}",
                update_command.domain_ident,
                update_command.register_domain_elf_ident,
                update_command.domain_type
            )
            .as_bytes()
            .to_vec(),
        }
    }
}

#[derive(Debug)]
pub enum Response {
    Ok(usize),
    Receive(usize, usize, usize),
}

impl Response {
    pub fn to_bytes(&self) -> Vec<u8> {
        match self {
            Response::Ok(id) => format!("ok:{}", id).as_bytes().to_vec(),
            Response::Receive(id, data_id, bytes) => {
                format!("receive:{}:{}:{}", id, data_id, bytes)
                    .as_bytes()
                    .to_vec()
            }
        }
    }
    pub fn parse(data: &[u8]) -> Option<Response> {
        let mut iter = data.splitn(2, |&x| x == b':');
        let response = iter.next()?;
        let response = core::str::from_utf8(response).ok()?;
        match response {
            "ok" => {
                let id = iter.next()?;
                let id = core::str::from_utf8(id).ok()?;
                let id = id.parse::<usize>().ok()?;
                Some(Response::Ok(id))
            }
            "receive" => {
                let data = iter.next()?;
                let mut iter = data.splitn(3, |&x| x == b':');
                let id = iter.next()?;
                let id = core::str::from_utf8(id).ok()?;
                let id = id.parse::<usize>().ok()?;
                let data_id = iter.next()?;
                let data_id = core::str::from_utf8(data_id).ok()?;
                let data_id = data_id.parse::<usize>().ok()?;
                let bytes = iter.next()?;
                let bytes = core::str::from_utf8(bytes).ok()?;
                let bytes = bytes.parse::<usize>().ok()?;
                Some(Response::Receive(id, data_id, bytes))
            }
            _ => None,
        }
    }
}

// start : domain_ident : DomainTypeRaw : domain size
// send  : id           : data_id       : bytes          : data
// stop  : id

// ok      : id
// receive : id           : data_id       : bytes

// update : old_domain_ident : new_domain_ident : DomainTypeRaw
