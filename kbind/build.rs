use std::{env, path::PathBuf};

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

fn main() {
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=KDIR");
    println!("cargo:rerun-if-env-changed=c_flags");
    println!("cargo:rerun-if-changed=src/bindings_helper.h");

    let kernel_dir = env::var("KDIR");
    if kernel_dir.is_err() {
        return;
    }
    let kernel_dir = kernel_dir.unwrap();
    let mut kernel_cflags = env::var("c_flags").expect("Add 'export c_flags' to Kbuild");
    kernel_cflags = kernel_cflags.replace("-mfunction-return=thunk-extern", "");
    kernel_cflags = kernel_cflags.replace("-fzero-call-used-regs=used-gpr", "");
    kernel_cflags = kernel_cflags.replace("-fconserve-stack", "");
    kernel_cflags = kernel_cflags.replace("-mrecord-mcount", "");
    kernel_cflags = kernel_cflags.replace("-Wno-maybe-uninitialized", "-Wno-uninitialized");
    kernel_cflags = kernel_cflags.replace("-Wno-alloc-size-larger-than", "");
    kernel_cflags = kernel_cflags.replace("-Wimplicit-fallthrough=5", "-Wimplicit-fallthrough");
    kernel_cflags = kernel_cflags.replace("-fsanitize=bounds-strict", "");
    kernel_cflags = kernel_cflags.replace("-ftrivial-auto-var-init=zero", "");

    let kbuild_cflags_module =
        env::var("KBUILD_CFLAGS_MODULE").expect("Must be invoked from kernel makefile");
    let cflags = format!("{} {}", kernel_cflags, kbuild_cflags_module);
    let kernel_args = prepare_cflags(&cflags, &kernel_dir);
    let target = env::var("TARGET").unwrap();

    generate_header(&kernel_args, &target);
}

fn generate_header(kernel_args: &Vec<String>, target: &str) {
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
    builder = builder.header("src/bindings_helper.h");
    let bindings = builder.generate().expect("Unable to generate bindings");
    // let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    let out_path = PathBuf::from("src");
    bindings
        .write_to_file(out_path.join("bindings_c.rs"))
        .expect("Couldn't write bindings!");
}
