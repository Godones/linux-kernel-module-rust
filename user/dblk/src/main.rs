use std::{
    fs::OpenOptions,
    io::{Read, Seek},
    os::unix::fs::OpenOptionsExt,
};

use core_affinity::CoreId;
use domain_helper::{DomainHelperBuilder, DomainTypeRaw};

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() != 2 {
        println!("Usage: dblk [load]/[unload]/[reload]/[update]");
        return;
    }
    let option = argv[1].as_str();
    match option {
        "load" => {
            load_block_device_domain();
        }
        "unload" => {
            unload_block_device_domain();
        }
        "reload" => {
            reload_block_device_domain();
        }
        "test" => {
            run_block_device_domain_test();
        }
        "update" => {
            update_block_device_domain();
        }
        _ => {
            println!("Usage: dblk [load]/[unload]/[reload]/[test]");
            return;
        }
    }
}

fn load_block_device_domain() {
    println!("Load block device domain");
    let builder = DomainHelperBuilder::new()
        .ty(DomainTypeRaw::BlockDeviceDomain)
        .domain_name("block_device")
        .domain_file_name("rnull")
        .domain_register_ident("rnull");
    builder.clone().register_domain_file().unwrap();
    builder.clone().load_domain().unwrap();
    println!("Load block device domain successfully");
}

fn unload_block_device_domain() {
    println!("Unload block device domain");
    DomainHelperBuilder::new()
        .ty(DomainTypeRaw::BlockDeviceDomain)
        .domain_name("block_device")
        .unload_domain()
        .unwrap();
    println!("Unload block device domain successfully");
}

fn reload_block_device_domain() {
    println!("Reload block device domain");
    DomainHelperBuilder::new()
        .ty(DomainTypeRaw::BlockDeviceDomain)
        .domain_name("block_device")
        .domain_register_ident("rnull")
        .load_domain()
        .unwrap();
    println!("Reload block device domain successfully");
}

fn update_block_device_domain() {
    println!("Update block device domain");
    let builder = DomainHelperBuilder::new()
        .ty(DomainTypeRaw::BlockDeviceDomain)
        .domain_file_name("rnull")
        .domain_name("block_device")
        .domain_register_ident("rnull");

    builder.clone().register_domain_file().unwrap();
    builder.clone().update_domain().unwrap();

    println!("Update block device domain successfully");
}

fn run_block_device_domain_test() {
    {
        let mut file = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECT)
            .open("/dev/drnullb0")
            .expect("Can't open");
        let mut buf = Vec::with_capacity(1024 * 128); // 32KB
        buf.resize(1024 * 128, 0);
        // let mut buf = [0;8096];
        let start = std::time::Instant::now();
        let r = file.read(&mut buf).unwrap();
        println!("Read {} bytes in {:?}", r, start.elapsed());
    }

    let thread_num = 8;
    let mut threads = vec![];
    for i in 0..thread_num {
        let thread = std::thread::spawn(move || {
            let id = CoreId { id: i + 4 };
            let res = core_affinity::set_for_current(id);
            assert!(res);

            let mut file = OpenOptions::new()
                .read(true)
                // .custom_flags(libc::O_DIRECT)
                .open("/dev/drnullb0")
                .expect("Can't open");
            let mut buf = Vec::with_capacity(1024 * 128); // 32KB
            buf.resize(1024 * 128, 0);
            let mut count = 0;
            let start = std::time::Instant::now();
            loop {
                let r = file.read(&mut buf).unwrap();
                if r == 0 {
                    file.rewind().expect("Can't rewind");
                }
                count += r;
                if start.elapsed().as_secs() > 10 {
                    println!(
                        "Thread {} read {} KB in {:?}",
                        i,
                        count / 1024,
                        start.elapsed()
                    );
                    break;
                }
            }
        });
        threads.push(thread);
    }
    for thread in threads {
        thread.join().unwrap();
    }
    println!("Read block device domain successfully");
}
