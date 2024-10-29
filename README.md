# Linux Kernel Module in Rust

Old introduction: See [old readme](./README_OLD.md).


## System requirements

- kernel version: 6.6 or 6.8



## Building hello-world

1. Install clang, kernel headers, and the `rust-src` and `rustfmt` components
   from `rustup`:

```
apt-get install llvm clang linux-headers-"$(uname -r)" # or the equivalent for your OS
rustup component add --toolchain=nightly rust-src rustfmt
```

2. cd to one of the examples

```
cd tests/hello-world
```

3. Build the kernel module using the Linux kernel build system (kbuild), this
   will invoke `cargo` to build the Rust code

```
make
```

4. Load and unload the module!

```
sudo insmod helloworld.ko
sudo rmmod helloworld
dmesg | tail
```


## Reference
