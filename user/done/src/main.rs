use std::{fs::OpenOptions, io::Write, sync::Arc, thread::sleep, time::Duration};

use domain_helper::{DomainHelperBuilder, DomainTypeRaw};
use spin::Mutex;

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() != 2 {
        println!("Usage: done [new]/[test]");
        return;
    }
    let option = argv[1].as_str();
    match option {
        "new" => {
           update_to_new();
        }
        "test" => {
            println!("Run null device domain test");
            run_log_domain_test();
        }
        _ => {
            println!("Usage: done [new]/[test]");
            return;
        }
    }
}

fn update_to_new() {
    println!("Register and update null device domain");
    let builder = DomainHelperBuilder::new()
        .ty(DomainTypeRaw::EmptyDeviceDomain)
        .domain_name("empty_device")
        .domain_file_name("null")
        .domain_register_ident("null");
    builder.clone().register_domain_file().unwrap();
    builder.clone().update_domain().unwrap();
    println!("Register and update null device domain to new version successfully");
}

fn run_log_domain_test() {
    const PATH: &str = "/proc/sys/rust/domain/one";
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
                let start = std::time::Instant::now();
                // Pin this thread to a single CPU core.
                let res = core_affinity::set_for_current(id);
                if res {
                    println!("Thread {} is running on core {}", id.id, id.id);
                    loop {
                        let mut file = file.lock();
                        let r = file.write(format!("I'm Thread {}", id.id).as_bytes());
                        println!("Thread {} write to file: {:?}", id.id, r);
                        let now = std::time::Instant::now();
                        // 75
                        if now.duration_since(start) > Duration::from_millis(200) {
                            println!("Thread {} is done", id.id);
                            break;
                        }
                        // sleep(Duration::from_millis(5));
                    }
                }
            });
            handlers.push(thread);
        }
    }
    let updater = std::thread::spawn(move || {
        sleep(Duration::from_millis(10));
        update_to_new();
        // sleep(Duration::from_millis(10));
        // update_to_old();
    });
    handlers.push(updater);
    for handle in handlers.into_iter() {
        handle.join().unwrap();
    }
}
