use std::{env, fs::OpenOptions, io::Write, path::PathBuf, process::Command};

#[allow(unused)]
const OPAQUE_TYPES: &[&str] = &[
    // These need to be opaque because they're both packed and aligned, which rustc
    // doesn't support yet. See https://github.com/rust-lang/rust/issues/59154
    // and https://github.com/rust-lang/rust-bindgen/issues/1538
    "desc_struct",
    "xregs_state",
];

// Takes the CFLAGS from the kernel Makefile and changes all the include paths to be absolute
// instead of relative.
fn prepare_cflags(cflags: &str, kernel_dir: &str) -> Vec<String> {
    let cflag_parts = shlex::split(&cflags).unwrap();
    let mut cflag_iter = cflag_parts.iter();
    let mut kernel_args = vec![];
    while let Some(arg) = cflag_iter.next() {
        if arg.starts_with("-I") && !arg.starts_with("-I/") {
            kernel_args.push(format!("-I{}/{}", kernel_dir, &arg[2..]));
        } else if arg == "-include" {
            kernel_args.push(arg.to_string());
            let include_path = cflag_iter.next().unwrap();
            if include_path.starts_with('/') {
                kernel_args.push(include_path.to_string());
            } else {
                kernel_args.push(format!("{}/{}", kernel_dir, include_path));
            }
        } else {
            kernel_args.push(arg.to_string());
        }
    }
    kernel_args
}

fn handle_kernel_version_cfg() {
    // read kernel version
    // if kernel version is less than 6.6, add KVER_LESS_6_6 to cfg
    // if kernel version is 6.6 or newer, add KVER_6_6_OR_NEWER to cfg
    let kernel_version = exec("uname", &["-r"]);
    let kernel_version = kernel_version.trim().split("-").next().unwrap();
    let kernel_version = kernel_version.split(".").collect::<Vec<&str>>();
    let kernel_version = (
        kernel_version[0].parse::<u32>().unwrap(),
        kernel_version[1].parse::<u32>().unwrap(),
    );
    if kernel_version.0 < 6 {
        panic!("The kernel version is less than 6.*");
    }

    println!("cargo:rustc-check-cfg=cfg(KVER_6_6_OR_NEWER)");
    println!("cargo:rustc-check-cfg=cfg(KVER_LESS_6_6)");
    let cfg = if kernel_version.0 == 6 && kernel_version.1 < 6 {
        "KVER_LESS_6_6"
    } else {
        "KVER_6_6_OR_NEWER"
    };
    println!("cargo:rustc-cfg={}", cfg);
}

fn main() {
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=KDIR");
    println!("cargo:rerun-if-env-changed=c_flags");

    let kernel_dir = env::var("KDIR");
    if kernel_dir.is_err() {
        return;
    }

    kallsyms_lookup_name();
    handle_kernel_version_cfg();

    let kernel_dir = kernel_dir.unwrap();
    let mut kernel_cflags = env::var("c_flags").expect("Add 'export c_flags' to Kbuild");
    kernel_cflags = kernel_cflags.replace("-mfunction-return=thunk-extern", "");
    kernel_cflags = kernel_cflags.replace("-fzero-call-used-regs=used-gpr", "");
    kernel_cflags = kernel_cflags.replace("-fconserve-stack", "");
    kernel_cflags = kernel_cflags.replace("-mrecord-mcount", "");
    kernel_cflags = kernel_cflags.replace("-Wno-maybe-uninitialized", "-Wno-uninitialized");
    kernel_cflags = kernel_cflags.replace("-Wno-alloc-size-larger-than", "");
    kernel_cflags = kernel_cflags.replace("-Wimplicit-fallthrough=5", "-Wimplicit-fallthrough");

    let kbuild_cflags_module =
        env::var("KBUILD_CFLAGS_MODULE").expect("Must be invoked from kernel makefile");

    let cflags = format!("{} {}", kernel_cflags, kbuild_cflags_module);
    let kernel_args = prepare_cflags(&cflags, &kernel_dir);

    let target = env::var("TARGET").unwrap();

    let mut builder = bindgen::Builder::default()
        .use_core()
        .ctypes_prefix("core::ffi")
        .derive_default(true)
        .size_t_is_usize(true)
        .layout_tests(false)
        .enable_function_attribute_detection();

    builder = builder.clang_arg(format!("--target={}", target));
    for arg in kernel_args.iter() {
        builder = builder.clang_arg(arg.clone());
    }
    builder = builder.opaque_type("alt_instr");

    println!("cargo:rerun-if-changed=src/bindings_helper.h");
    builder = builder.header("src/bindings_helper.h");
    let bindings = builder.generate().expect("Unable to generate bindings");

    // let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_path = PathBuf::from("src");
    bindings
        .write_to_file(out_path.join("bindings_c.rs"))
        .expect("Couldn't write bindings!");

    let mut builder = cc::Build::new();
    builder.compiler(env::var("CC").unwrap_or_else(|_| "clang".to_string()));
    builder.target(&target);
    builder.warnings(false);
    println!("cargo:rerun-if-changed=src/helpers.c");
    builder.file("src/helpers.c");
    for arg in kernel_args.iter() {
        builder.flag(&arg);
    }
    builder.remove_flag("-pg");
    builder.compile("helpers");
}

const INCLUDE_FUNCS: &[&str] = &["module_alloc", "module_memfree", "set_memory_x"];
pub fn kallsyms_lookup_name() {
    let mut env_file = OpenOptions::new()
        .write(true)
        .create(true)
        .open("./src/env.rs")
        .unwrap();
    let ret = exec(
        "sh",
        &[
            "-c",
            "sudo cat /proc/kallsyms | grep -E 'module_alloc|module_memfree|set_memory_x'",
        ],
    );
    let lines = ret.split("\n");
    for line in lines {
        if line.is_empty() {
            continue;
        }
        let mut parts = line.split(" ");
        let addr = parts.next().unwrap();
        let name = parts.last().unwrap();
        if INCLUDE_FUNCS.contains(&name) {
            let addr = usize::from_str_radix(addr, 16).unwrap();
            println!("{}: {:#x}", name.to_uppercase(), addr);
            // env::set_var(format!("{}_ADDR", name.to_uppercase()), format!("{:#x}", addr));
            // println!("cargo:rustc-env={}={:#x}", name.to_uppercase(), addr);
            env_file
                .write(
                    format!(
                        "pub const {}_ADDR: usize = {:#x};\n",
                        name.to_uppercase(),
                        addr
                    )
                    .as_bytes(),
                )
                .unwrap();
        } else {
            println!("skip: {}", name);
        }
    }
}

fn exec(cmd: &str, args: &[&str]) -> String {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .expect("failed to execute cmd");

    String::from_utf8(output.stdout).unwrap()
}
