use std::{fs::OpenOptions, io::Write, sync::Arc, thread::sleep, time::Duration};

use domain_helper::{DomainHelperBuilder, DomainTypeRaw};
use spin::mutex::TicketMutex;

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() != 2 {
        println!("Usage: dlog [new]/[old]/[test]");
        return;
    }
    let option = argv[1].as_str();
    match option {
        "new" => {
            update_to_new();
        }
        "old" => {
            update_to_old();
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

fn update_to_new() {
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

fn update_to_old() {
    println!("Register and update logger domain to old version");
    DomainHelperBuilder::new()
        .ty(DomainTypeRaw::LogDomain)
        .domain_name("logger")
        .domain_register_ident("logger")
        .update_domain()
        .unwrap();
    println!("Register and update logger domain to old version successfully");
}
fn run_log_domain_test() {
    const PATH: &str = "/proc/sys/rust/domain/entropy";
    const THREAD_NUM: usize = 4;
    let file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(PATH)
        .unwrap();
    let file = Arc::new(TicketMutex::<_>::new(file));
    let mut handlers = vec![];
    // Create a thread for each active CPU core.
    for id in 0..THREAD_NUM {
        let file = file.clone();
        let thread = std::thread::spawn(move || {
            let start = std::time::Instant::now();
            println!("Thread {} is running ", id);
            loop {
                let mut file = file.lock();
                file.write(format!("I'm Thread {}", id).as_bytes()).unwrap();
                drop(file);
                // sleep(Duration::from_secs(1));
                let now = std::time::Instant::now();
                // 75
                if now.duration_since(start) > Duration::from_millis(100) {
                    println!("Thread {} is done", id);
                    break;
                }
                sleep(Duration::from_millis(5));
            }
        });
        handlers.push(thread);
    }
    let updater = std::thread::spawn(move || {
        sleep(Duration::from_millis(10));
        update_to_new();
        sleep(Duration::from_millis(10));
        update_to_old();
    });
    handlers.push(updater);

    for handle in handlers.into_iter() {
        handle.join().unwrap();
    }
}
