use domain_helper::{DomainHelperBuilder, DomainTypeRaw};

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() != 2 {
        println!("Usage: dblk [load]/[unload]/[test]");
        return;
    }
    let option = argv[1].as_str();
    match option {
        "load" => {
            println!("Load block device domain");
            load_block_device_domain();
        }
        "unload" => {
            println!("Unload block device domain");
            unload_block_device_domain();
        }
        "test" => {
            println!("Run block device domain test");
            run_block_device_domain_test();
        }
        _ => {
            println!("Usage: dblk [load]/[unload]/[test]");
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

fn run_block_device_domain_test() {}
