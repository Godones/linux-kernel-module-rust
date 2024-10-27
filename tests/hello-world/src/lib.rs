#![no_std]
extern crate alloc;

use alloc::{string::String, vec::Vec};

use kernel::{error::KernelResult as Result, *};

module! {
    type: RustMinimal,
    name: "rust_minimal",
    author: "Rust for Linux Contributors",
    description: "Rust minimal sample",
    license: "GPL",
}

struct RustMinimal {
    numbers: Vec<i32>,
}

impl kernel::Module for RustMinimal {
    fn init(_module: &'static ThisModule) -> Result<Self> {
        kernel::logger::init_logger();
        pr_info!("Rust minimal sample (init)\n");
        pr_info!("Am I built-in? {}\n", !cfg!(MODULE));

        log::info!("Hello, world!");
        let mut numbers = Vec::new();
        numbers.push(72);
        numbers.push(108);
        numbers.push(200);

        let res = catch_unwind(|| {
            bar();
        });
        println!("catch_unwind result: {:?}", res);
        println!("The panic has been caught");
        Ok(RustMinimal { numbers })
    }
}

impl Drop for RustMinimal {
    fn drop(&mut self) {
        pr_info!("My numbers are {:?}\n", self.numbers);
        pr_info!("Rust minimal sample (exit)\n");
    }
}

#[derive(Debug)]
struct PrintOnDrop(String);

impl Drop for PrintOnDrop {
    fn drop(&mut self) {
        println!("dropped: {:?}", self.0);
    }
}

fn foo() {
    panic!("panic at foo");
}

#[inline(never)]
fn bar() {
    use alloc::string::String;
    let p1 = PrintOnDrop(String::from("PrintOnDrop1"));
    let p2 = PrintOnDrop(String::from("PrintOnDrop2"));
    println!("p1: {:?}, p2: {:?}", p1, p2);
    foo()
}
