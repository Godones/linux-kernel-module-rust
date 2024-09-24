use std::{
    fs,
    fs::OpenOptions,
    io::{Read, Write},
    path::Path,
};

use command::{Command, Response, SendCommand, StartCommand, StopCommand};

const PATH: &str = "/proc/sys/rust/domain/command";
const DOMAIN_TYPE: &[&str] = &["disk", "init"];

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
pub fn register_domain(name: &str, ty: u8, register_ident: &str) {
    let path = find_path(name).expect("Domain not found");
    let mut file = fs::File::open(path).unwrap();
    let file_size = file.metadata().unwrap().len();

    // send start command
    let start_command = StartCommand {
        domain_ident: register_ident,
        domain_type: ty,
        domain_size: file_size as usize,
    };
    let start_command = Command::Start(start_command);
    let start_command = start_command.to_bytes();

    write_to_channel(&start_command);

    let res_buf = read_from_channel();
    let response = Response::parse(&res_buf).expect("Parse response failed");
    println!("Response: {:?}", response);
    let id = match response {
        Response::Ok(id) => id,
        _ => {
            println!("Invalid response");
            return;
        }
    };
    // send file data
    let mut count = 0;
    let mut buf = [0; 512];
    let mut data_id = 0;
    while count < file_size as usize {
        let res = file.read(&mut buf).expect("Read file failed");
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
        write_to_channel(&send_command);
        // read response to make sure the data is sent
        let res_buf = read_from_channel();
        let response = Response::parse(&res_buf).expect("Parse response failed");
        println!("Response: {:?}", response);

        match response {
            Response::Receive(id, data_id, bytes) => {
                if id != id {
                    println!("Invalid id");
                    return;
                }
                if data_id != data_id {
                    println!("Invalid data id");
                    return;
                }
                if bytes != res {
                    println!("Invalid data length");
                    return;
                }
            }
            _ => {
                println!("Invalid response");
                return;
            }
        }
        count += res;
        data_id += 1;
    }
    // send stop command
    let stop_command = StopCommand { id: id as u64 };
    let stop_command = Command::Stop(stop_command);
    let stop_command = stop_command.to_bytes();
    write_to_channel(&stop_command);

    let res_buf = read_from_channel();
    let response = Response::parse(&res_buf).expect("Parse response failed");
    println!("Response: {:?}", response);
    let id = match response {
        Response::Ok(id) => id,
        _ => {
            println!("Invalid response");
            return;
        }
    };
    println!("Domain registered: {}", id);
}

fn open_channel() -> fs::File {
    OpenOptions::new()
        .write(true)
        .read(true)
        .open(PATH)
        .unwrap()
}

fn write_to_channel(data: &[u8]) {
    let mut file = open_channel();
    file.write(data).expect("Write failed");
}

fn read_from_channel() -> Vec<u8> {
    let mut file = open_channel();
    let mut buf = [0u8; 64];
    let res = file.read(&mut buf).expect("Read failed");
    buf[..res].to_vec()
}
