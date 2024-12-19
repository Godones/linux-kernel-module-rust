use std::{
    fs,
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};

use command::{
    Command, LoadCommand, Response, SendCommand, StartCommand, StopCommand, UnloadCommand,
    UpdateCommand,
};

use super::Result;
use crate::{DOMAIN_TYPE, PATH};

fn find_path(name: &str) -> Option<String> {
    for ty in DOMAIN_TYPE {
        let file_path = format!("./build/{}/g{}", ty, name);
        let path = Path::new(&file_path);
        if path.exists() {
            return Some(file_path);
        }
    }
    None
}

/// Register a domain to the kernel
///
/// It will communicate with the kernel module using a file in /proc/sys/rust/domain/command
/// and send the domain file to the kernel module
pub fn register_domain(name: &str, ty: u8, register_domain_elf_ident: &str) -> Result<()> {
    let path = find_path(name).ok_or_else(|| format!("Domain file {} not found", name))?;
    let mut file = fs::File::open(path)?;
    let file_size = file.metadata()?.len();

    // send start command
    let start_command = StartCommand {
        register_domain_elf_ident,
        domain_type: ty,
        domain_size: file_size as usize,
    };
    let start_command = Command::Start(start_command);
    let start_command = start_command.to_bytes();

    write_to_channel(&start_command)?;

    let res_buf = read_from_channel()?;
    let response = Response::parse(&res_buf).ok_or("Parse response failed")?;
    // println!("Response: {:?}", response);
    let id = match response {
        Response::Ok(id) => id,
        _ => {
            return Err("Invalid response".into());
        }
    };
    // send file data
    let mut count = 0;
    let mut buf = [0; 512];
    let mut data_id = 0;
    while count < file_size as usize {
        let res = file.read(&mut buf)?;
        if res == 0 {
            break;
        }
        let send_command = Command::Send(SendCommand {
            id: id as u64,
            data_id,
            bytes: res,
            data: &buf[..res],
        });
        let send_command = send_command.to_bytes();
        write_to_channel(&send_command)?;
        // read response to make sure the data is sent
        let res_buf = read_from_channel()?;
        let response = Response::parse(&res_buf).ok_or("Parse response failed")?;
        // println!("Response: {:?}", response);

        match response {
            Response::Receive(id, data_id, bytes) => {
                if id != id {
                    return Err("Invalid id".into());
                }
                if data_id != data_id {
                    return Err("Invalid data id".into());
                }
                if bytes != res {
                    return Err("Invalid data length".into());
                }
            }
            _ => {
                return Err("Invalid response".into());
            }
        }
        count += res;
        data_id += 1;
    }
    // send stop command
    let stop_command = StopCommand { id: id as u64 };
    let stop_command = Command::Stop(stop_command);
    let stop_command = stop_command.to_bytes();
    write_to_channel(&stop_command)?;

    let res_buf = read_from_channel()?;
    let response = Response::parse(&res_buf).ok_or("Parse response failed")?;
    // println!("Response: {:?}", response);
    let id = match response {
        Response::Ok(id) => id,
        _ => {
            return Err("Invalid response".into());
        }
    };
    println!("Domain registered: {}", id);
    Ok(())
}

pub fn update_domain(old_ident: &str, register_domain_elf_ident: &str, ty: u8) -> Result<()> {
    let update_command = Command::Update(UpdateCommand {
        domain_ident: old_ident,
        register_domain_elf_ident,
        domain_type: ty,
    });
    let update_command = update_command.to_bytes();
    write_to_channel(&update_command)?;

    let res_buf = read_from_channel()?;
    let response = Response::parse(&res_buf).ok_or("Parse response failed")?;
    println!("update_domain: Response: {:?}", response);
    Ok(())
}

pub fn load_domain(register_domain_elf_ident: &str, domain_ident: &str, ty: u8) -> Result<()> {
    let load_command = Command::Load(LoadCommand {
        register_domain_elf_ident,
        domain_ident,
        domain_type: ty,
    });
    let load_command = load_command.to_bytes();
    write_to_channel(&load_command)?;

    let res_buf = read_from_channel()?;
    let response = Response::parse(&res_buf).ok_or("Parse response failed")?;
    println!("load_domain: Response: {:?}", response);
    Ok(())
}

pub fn unload_domain(domain_ident: &str) -> Result<()> {
    let unload_command = Command::Unload(UnloadCommand { domain_ident });
    let unload_command = unload_command.to_bytes();
    write_to_channel(&unload_command)?;

    let res_buf = read_from_channel()?;
    let response = Response::parse(&res_buf).ok_or("Parse response failed")?;
    println!("unload_domain: Response: {:?}", response);
    Ok(())
}

fn open_channel() -> Result<fs::File> {
    let file = OpenOptions::new().write(true).read(true).open(PATH)?;
    Ok(file)
}

fn write_to_channel(data: &[u8]) -> Result<()> {
    let mut file = open_channel()?;
    file.write(data)?;
    Ok(())
}

fn read_from_channel() -> Result<Vec<u8>> {
    let mut file = open_channel()?;
    let mut buf = [0u8; 64];
    let res = file.read(&mut buf)?;
    Ok(buf[..res].to_vec())
}
