use std::{
    fs::OpenOptions,
    io::{Read, Seek, Write},
    sync::Arc,
    thread::sleep,
    time::Duration,
};

use domain_helper::{DomainHelperBuilder, DomainTypeRaw};
use spin::Mutex;

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() < 2 {
        println!("Usage: done [new]/[test]/[panic]/[old]");
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
        "thread" => {
            let num = argv[2].parse::<usize>().unwrap();
            println!("Run null device domain test with {} threads", num);
            multi_thread_test(num);
        }
        "test" => {
            println!("Run null device domain test");
            run_log_domain_test();
        }
        "panic" => {
            panic_test();
        }
        _ => {
            println!("Usage: done [new]/[test]/[panic]/[old]");
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

fn update_to_old() {
    println!("Register and update null device domain to old version");
    DomainHelperBuilder::new()
        .ty(DomainTypeRaw::EmptyDeviceDomain)
        .domain_name("empty_device")
        .domain_register_ident("xnull")
        .update_domain()
        .unwrap();
    println!("Register and update null device domain to old version successfully");
}

fn multi_thread_test(thread_num: usize) {
    let mut threads = Vec::new();
    for i in 0..thread_num {
        let thread = std::thread::spawn(move || {
            let start = std::time::Instant::now();
            let mut file = OpenOptions::new()
                .write(true)
                .read(true)
                .open(PATH)
                .unwrap();
            loop {
                let mut buf = [0u8; 100];
                let r = file.read(&mut buf).unwrap();
                assert!(r > 0);
                file.rewind().unwrap();
                if start.elapsed().as_secs() > 5 {
                    break;
                }
            }
            println!("Thread {} is done, run {}sec", i, start.elapsed().as_secs());
        });
        threads.push(thread);
    }

    // let updater = std::thread::spawn(move || {
    //     sleep(Duration::from_secs(5));
    //     update_to_new();
    // });
    // threads.push(updater);
    for handle in threads.into_iter() {
        handle.join().unwrap();
    }
}

fn panic_test() {
    let mut file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(PATH)
        .unwrap();
    file.write(b"panic test").unwrap();
}
const PATH: &str = "/proc/sys/rust/domain/one";
fn run_log_domain_test() {
    const THREAD_NUM: usize = 4;
    let file = OpenOptions::new()
        .write(true)
        .read(true)
        .open(PATH)
        .unwrap();
    let file = Arc::new(Mutex::new(file));

    let mut handlers = vec![];

    for id in 0..THREAD_NUM {
        let file = file.clone();
        let thread = std::thread::spawn(move || {
            let start = std::time::Instant::now();
            println!("Thread {} is running", id);
            loop {
                let mut file = file.lock();
                let r = file.write(format!("I'm Thread {}", id).as_bytes());
                drop(file);
                println!("Thread {} write to file: {:?}", id, r);
                let now = std::time::Instant::now();
                // 75
                if now.duration_since(start) > Duration::from_millis(100) {
                    println!("Thread {} is done", id);
                    break;
                }
                // sleep(Duration::from_millis(5));
            }
        });
        handlers.push(thread);
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
