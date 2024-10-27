use std::{fs, path::Path};

use crate::subcommand::{Arch, Config, DOMAIN_SET};

pub fn build_single(name: &str, log: &str, arch: Option<String>) {
    let domain_list = fs::read_to_string("./domains/domain-list.toml").unwrap();
    let config: Config = toml::from_str(&domain_list).unwrap();
    let all_members = config.domains.get("members").unwrap();
    let r_name = name;
    if !all_members.contains(&r_name.to_string()) {
        println!(
            "Domain [{}] is not in the members list, skip building",
            r_name
        );
        return;
    }
    let init_members = config.domains.get("init_members").unwrap();
    if init_members.contains(&r_name.to_string()) {
        build_domain(name, log.to_string(), "init", arch.into());
    } else {
        let disk_members = config.domains.get("disk_members").unwrap();
        if disk_members.contains(&r_name.to_string()) {
            build_domain(name, log.to_string(), "disk", arch.into());
        } else {
            println!(
                "Domain [{}] is not in the init or disk members list, skip building",
                r_name
            );
        }
    }
}

pub fn build_domain(name: &str, log: String, dir: &str, arch: Arch) {
    // change the directory to the domain project
    // run cargo build
    println!("Building domain [{}] project", name);
    for ty in DOMAIN_SET {
        let path = format!("./domains/{}/{}/g{}/Cargo.toml", ty, name, name);
        let path = Path::new(&path);
        if path.exists() {
            let path = format!("./{}/{}/g{}/Cargo.toml", ty, name, name);
            let path = Path::new(&path);
            println!("Cargo.toml found in {}, path is {}", ty, path.display());
            println!("The target arch is {:?}", arch);
            let _cmd = std::process::Command::new("cargo")
                .arg("build")
                .arg("--release")
                .env("LOG", log)
                .arg("--manifest-path")
                .arg(path)
                .arg("--target")
                .arg(arch.target_json())
                .arg("--target-dir")
                .arg("../target")
                .current_dir("./domains")
                .status()
                .expect("failed to execute cargo build");
            println!("Build domain [{}] project success", name);
            std::process::Command::new("cp")
                .arg(format!(
                    "./target/{}/release/g{}",
                    arch.target_build(),
                    name
                ))
                .arg(format!("./build/{}/g{}", dir, name))
                .status()
                .expect("failed to execute cp");
            println!("Copy domain [{}] project success", name);
            return;
        }
    }
}

pub fn build_all(log: String, arch: Option<String>) {
    let domain_list = fs::read_to_string("./domains/domain-list.toml").unwrap();
    let config: Config = toml::from_str(&domain_list).unwrap();
    println!("Start building all domains");
    let all_members = config.domains.get("members").unwrap().clone();
    let init_members = config.domains.get("init_members").unwrap().clone();
    for domain_name in init_members {
        if !all_members.contains(&domain_name) {
            println!(
                "Domain [{}] is not in the members list, skip building",
                domain_name
            );
            continue;
        }
        let value = log.to_string();
        // pool.spawn(move || build_domain(&domain_name, value, "init"));
        // build_domain(&domain_name, value, "init")
        let target_arch = arch.clone().into();
        build_domain(&domain_name, value, "init", target_arch);
    }
    let disk_members = config.domains.get("disk_members").unwrap().clone();
    if !disk_members.is_empty() {
        for domain_name in disk_members {
            if !all_members.contains(&domain_name) {
                println!(
                    "Domain [{}] is not in the members list, skip building",
                    domain_name
                );
                continue;
            }
            let value = log.to_string();
            // pool.spawn(move || build_domain(&domain_name, value, "disk"));
            // build_domain(&domain_name, value, "disk")
            let target_arch = arch.clone().into();
            build_domain(&domain_name, value, "disk", target_arch)
        }
    }
}
