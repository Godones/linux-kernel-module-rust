use std::{fs::OpenOptions, io::Write, sync::Arc, thread::sleep, time::Duration};

use domain_helper::{DomainHelperBuilder, DomainTypeRaw};
use spin::Mutex;

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() != 2 {
        println!("Usage: dlog [new]/[old]/[test]");
        return;
    }
    let option = argv[1].as_str();
    match option {
        "new" => {
            println!("Register and update xlogger domain");
            let builder = DomainHelperBuilder::new()
                .ty(DomainTypeRaw::LogDomain)
                .domain_name("logger")
                .domain_file_name("logger")
                .domain_register_ident("xlogger");
            builder.clone().register_domain_file().unwrap();
            builder.clone().update_domain().unwrap();
            println!("Register and update logger domain to new version successfully");
        }
        "old" => {
            DomainHelperBuilder::new()
                .ty(DomainTypeRaw::LogDomain)
                .domain_name("logger")
                .domain_register_ident("logger")
                .update_domain()
                .unwrap();
            println!("Register and update logger domain to old version successfully");
        }
        "test" => {
            println!("Run log domain test");
            run_log_domain_test();
        }
        _ => {
            println!("Usage: dlog [new]/[old]/[test]");
            return;
        }
    }
}

fn run_log_domain_test() {
    const PATH: &str = "/proc/sys/rust/domain/entropy";
    const THREAD_NUM: usize = 4;
    let file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(PATH)
        .unwrap();
    let file = Arc::new(Mutex::new(file));

    let mut handlers = vec![];

    // Retrieve the IDs of all active CPU cores.
    let core_ids = core_affinity::get_core_ids().unwrap();
    // Create a thread for each active CPU core.
    for id in core_ids.into_iter() {
        if id.id < THREAD_NUM {
            let file = file.clone();
            let thread = std::thread::spawn(move || {
                // Pin this thread to a single CPU core.
                let res = core_affinity::set_for_current(id);
                if res {
                    println!("Thread {} is running on core {}", id.id, id.id);
                    loop {
                        let mut file = file.lock();
                        file.write(format!("I'm Thread {}", id.id).as_bytes())
                            .unwrap();
                        sleep(Duration::from_secs(3));
                    }
                }
            });
            handlers.push(thread);
        }
    }

    for handle in handlers.into_iter() {
        handle.join().unwrap();
    }
}
