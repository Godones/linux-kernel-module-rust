use std::collections::BTreeMap;

use serde::Deserialize;

pub mod build;
pub mod clean;
pub mod fmt;
pub mod r#move;
pub mod new;

#[derive(Deserialize)]
pub struct Config {
    pub domains: BTreeMap<String, Vec<String>>,
}
static DOMAIN_SET: [&str; 3] = ["common", "fs", "drivers"];

#[derive(Debug)]
pub enum Arch {
    RV64(ArchPathInfo),
    X86_64(ArchPathInfo),
}
#[derive(Debug)]
pub struct ArchPathInfo {
    pub target_json: String,
    pub target_build: String,
}

impl From<Option<String>> for Arch {
    fn from(value: Option<String>) -> Self {
        match value {
            None => Arch::RV64(ArchPathInfo {
                target_json: "./riscv64.json".to_string(),
                target_build: "riscv64".to_string(),
            }),
            Some(arch) => match arch.as_str() {
                "riscv64" => Arch::RV64(ArchPathInfo {
                    target_json: format!("./{}.json", arch),
                    target_build: arch,
                }),
                "x86_64" => Arch::X86_64(ArchPathInfo {
                    target_json: format!("./{}.json", arch),
                    target_build: arch,
                }),
                arch => panic!("Unsupported arch {}", arch),
            },
        }
    }
}

impl Arch {
    pub fn target_json(&self) -> &str {
        match self {
            Arch::RV64(info) => &info.target_json,
            Arch::X86_64(info) => &info.target_json,
        }
    }

    pub fn target_build(&self) -> &str {
        match self {
            Arch::RV64(info) => &info.target_build,
            Arch::X86_64(info) => &info.target_build,
        }
    }
}
