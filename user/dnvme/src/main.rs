use domain_helper::{DomainHelperBuilder, DomainTypeRaw};

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    if argv.len() != 2 {
        println!("Usage: dnvme [load]/[unload]/[test]");
        return;
    }
    let option = argv[1].as_str();
    match option {
        "load" => {
            println!("Load nvme device domain");
            load_block_device_domain();
        }
        "unload" => {
            println!("Unload nvme device domain");
            unload_block_device_domain();
        }
        "test" => {
            println!("Run nvme device domain test");
            run_block_device_domain_test();
        }
        _ => {
            println!("Usage: dnvme [load]/[unload]/[test]");
            return;
        }
    }
}

fn load_block_device_domain() {
    println!("Load nvme device domain");
    let builder = DomainHelperBuilder::new()
        .ty(DomainTypeRaw::NvmeBlockDeviceDomain)
        .domain_name("nvme_device")
        .domain_file_name("rnvme")
        .domain_register_ident("rnvme");
    builder.clone().register_domain_file().unwrap();
    builder.clone().load_domain().unwrap();
    println!("Load nvme device domain successfully");
}

fn unload_block_device_domain() {
    println!("Unload nvme device domain");
    DomainHelperBuilder::new()
        .ty(DomainTypeRaw::NvmeBlockDeviceDomain)
        .domain_name("nvme_device")
        .unload_domain()
        .unwrap();
    println!("Unload nvme device domain successfully");
}

fn run_block_device_domain_test() {}
