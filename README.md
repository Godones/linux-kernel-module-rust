# Linux Kernel Module in Rust

Old introduction: See [old readme](./README_OLD.md).


## System requirements

- kernel version: 6.6 or 6.8

Source code: [linux 6.6](https://github.com/Godones/linux/tree/null_blk-6.6)

## Building all domain

To build all domain for x86_64 architecture, run:
```
cargo domain build-all -a x86_64
```
This will build all example domains located in the `domains/` directory.
- null(test domain)
- logger(logging domain)
- rnull(null block device domain)
- rnvme(nvme block device domain)


## Build and Run TCB
To build and run the TCB (Trusted Computing Base) module, execute the following commands:
```
make run
```


## logger domain
To load the logger domain module, execute the following commands:
```
cargo run -p dlog new
```
Then, you can test the logger domain by running:
```
cargo run -p dlog test
```
See dmesg for log messages.



## null blk device domain
To load or unload the null block device domain module, execute the following commands:
```
cargo run -p dblk load/unload
```
Then, you can test the null block device by running:
```
sudo ./target/debug/dblk test
```

## NVMe blk device domain
You need make sure that the kernel has not already loaded the default NVMe driver.

To load or unload the NVMe block device domain module, execute the following commands:
```
cargo run -p dnvme load/unload
```



## Documentation

[rust for linux](./doc/rust_for_linux.md)

## Evaluation Results
[Evaluation Results](./evaluation)
## Reference
