# Rust for Linux

记录rust for linux 的使用过程, 这里尝试在`riscv`平台启动，因为后期的代码与`riscv`相关。

## 编译

```
git clone --depth=1 git@github.com:Godones/linux.git -b rust-dev
```

`rust-dev` 分支是开发最快的分支，因此我们选择这个分支进行实验。

```
cd linux
# 从https://www.rust-lang.org安装rustup
rustup override set $(scripts/min-tool-version.sh rustc)	# rustc
rustup component add rust-src	# rust-src
sudo apt install clang llvm		# libclang
cargo install --locked --version $(scripts/min-tool-version.sh bindgen) bindgen	# bindgen
rustup component add rustfmt	# rustfmt
rustup component add clippy		# clippy
make LLVM=1 rustavailable		# 验证如上依赖安装无误，输出“Rust is available!”
```

```
# for amd64
make LLVM=1 menuconfig
	# Kernel hacking -> Sample kernel code -> Rust samples 选择一些模块
make LLVM=1 -j

# for aarch64
make ARCH=arm64 CLANG_TRIPLE=aarch64_linux_gnu LLVM=1 menuconfig
make ARCH=arm64 CLANG_TRIPLE=aarch64_linux_gnu LLVM=1 -j

# for riscv64
# 注： meuconfig时关闭kvm模块，否则内核有bug不能成功编译
make LLVM=1  ARCH=riscv defconfig
make ARCH=riscv LLVM=1 menuconfig
make ARCH=riscv LLVM=1 -j32

# 生成编辑器索引文件
make LLVM=1 ARCH=riscv rust-analyzer
bear -- make ARCH=riscv LLVM=1 -j12
```



### Linux on riscv

https://blog.csdn.net/youzhangjing_/article/details/129556309

1. 安装riscv的工具链:  riscv64-linux-gnu-
2. 安装qemu
3. 编译kernel

```
export ARCH=riscv
export CROSS_COMPILE=riscv64-linux-gnu-
 
make defconfig
make -j8
```

编译完成后，在arch/riscv/boot下生成Image

4. 制作rootfs，使用buildroot工具直接一步到位生成。编译完后，生成文件在output/images目录下
5. 将Image、rootfs.ext2拷贝到同一目录下，运行qemu命令

```
#!/bin/sh
 
qemu-system-riscv64 -M virt \
-kernel Image \
-append "rootwait root=/dev/vda ro" \
-drive file=rootfs.ext2,format=raw,id=hd0 \
-device virtio-blk-device,drive=hd0 \
-netdev user,id=net0 -device virtio-net-device,netdev=net0 -nographic
```



在wsl2中，编译buildroot可能会报错，原因是其会检查环境变量，但是PATH环境变量中包含windows的路径，里面包含空格。

解决办法是直接指定PATH变量进行编译

```
PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin make
```

https://blog.csdn.net/weixin_40837318/article/details/134328622



另一种途径：

1. 编译linux

```
make ARCH=riscv CROSS_COMPILE=riscv64-linux-gnu- defconfig
make ARCH=riscv CROSS_COMPILE=riscv64-linux-gnu- -j $(nproc)
```

2. 使用busybox构建根文件系统

```
git clone https://gitee.com/mirrors/busyboxsource
cd busyboxsource
export CROSS_COMPILE=riscv64-linux-gnu-
make defconfig
make menuconfig
这里启用了 Settings-->Build Options 里的 Build static binary (no shared libs) 选项
make -j $(nproc)
make install
```

3. 制作文件系统并新建一个启动脚本

```
$ cd ~
$ qemu-img create rootfs.img 1g
$ mkfs.ext4 rootfs.img
$ mkdir rootfs
$ sudo mount -o loop rootfs.img rootfs
$ cd rootfs
$ sudo cp -r ../busyboxsource/_install/* .
$ sudo mkdir proc sys dev etc etc/init.d
$ cd etc/init.d/
$ sudo touch rcS
$ sudo vi rcS
```

https://gitee.com/YJMSTR/riscv-linux/blob/master/articles/20220816-introduction-to-qemu-and-riscv-upstream-boot-flow.md



### 加载内核模块

在使用`make ARCH=riscv LLVM=1 menuconfig` 进行配置时，我们可以选择开启编译一些内核内置的rust模块

这些模块被编译后不会和内核镜像绑定在一起，因此我们需要手动将其拷贝到制作的文件系统中，在文件系统中，使用insmod相关的命令加载和卸载这些模块。

`make ARCH=riscv LLVM=1 modules_install INSTALL_MOD_PATH=?` 这个命令会把编译的内核模块拷贝到本机目录下，因此通常不使用

`make ARCH=riscv LLVM=1 modules` 只会编译选择的内核模块



### 构建ubuntu镜像

为了后续方便在系统上进行测试，构建一个ubuntu/debian镜像是必须的。这样一来就可以使用常见的性能测试工具。

依赖安装：

```
sudo apt install debootstrap qemu qemu-user-static binfmt-support
```

生成最小 bootstrap rootfs 

```
sudo debootstrap --arch=riscv64 --foreign jammy ./temp-rootfs http://mirrors.tuna.tsinghua.edu.cn/ubuntu-ports
```

拷贝qemu-riscv64-static到rootfs中

```
cp /usr/bin/qemu-riscv64-static ./temp-rootfs/usr/bin/
```

 chroot 和 debootstrap

```
wget https://raw.githubusercontent.com/ywhs/linux-software/master/ch-mount.sh
chmod 777 ch-mount.sh
./ch-mount.sh -m temp-rootfs/
# 执行脚本后，没有报错会进入文件系统，显示 I have no name ，这是因为还没有初始化。
debootstrap/debootstrap --second-stage
exit
./ch-mount.sh -u temp-rootfs/
./ch-mount.sh -m temp-rootfs/
```

这里如果在WSL2进行实验，可能会遇到无法chroot的情况。issue https://github.com/microsoft/WSL/issues/2103#issuecomment-1829496706给出了解决办法。

修改软件源：

```
deb http://mirrors.tuna.tsinghua.edu.cn/ubuntu-ports/ jammy main restricted universe multiverse
deb http://mirrors.tuna.tsinghua.edu.cn/ubuntu-ports/ jammy-updates main restricted universe multiverse
deb http://mirrors.tuna.tsinghua.edu.cn/ubuntu-ports/ jammy-backports main restricted universe multiverse
deb http://mirrors.tuna.tsinghua.edu.cn/ubuntu-ports/ jammy-security main restricted universe multiverse
```

安装常见的工具

```
apt-get update
apt-get install --no-install-recommends -y util-linux haveged openssh-server systemd kmod initramfs-tools conntrack ebtables ethtool iproute2 iptables mount socat ifupdown iputils-ping vim dhcpcd5 neofetch sudo chrony
```



制作ext4的镜像

```
dd if=/dev/zero of=rootfs_ubuntu_riscv.ext4 bs=1M count=4096
mkfs.ext4 rootfs_ubuntu_riscv.ext4
mkdir -p tmpfs
sudo mount -t ext4 rootfs_ubuntu_riscv.ext4 tmpfs/ -o loop
sudo cp -af temp-rootfs/* tmpfs/
sudo umount tmpfs
chmod 777 rootfs_ubuntu_riscv.ext4
```

使用ssh连接启动的qemu

https://copyright1999.github.io/2023/10/03/wsl%E5%A6%82%E4%BD%95ssh%E5%88%B0qemu/

启动`qemu`之后，还要在我们`qemu`模拟的`debian`中做好`ssh`相关的配置，在`/etc/ssh/sshd_config`中加上一句

```
PermitRootLogin yes
```

然后重启`ssh`相关服务

```
service ssh restart
```

设置内核模块输出到控制台

https://blog.csdn.net/dp__mcu/article/details/119887176

http://www.only2fire.com/archives/27.html

将linux编译的内核模块安装到文件系统中：

```
sudo make ARCH=riscv LLVM=1 modules_install INSTALL_MOD_PATH=../mnt/kmod
```



https://blog.csdn.net/jingyu_1/article/details/135822574

https://cloud.tencent.com/developer/article/1914743

https://blog.csdn.net/xuesong10210/article/details/129167731

### 编写内核模块

c内核模块https://linux-kernel-labs-zh.xyz/labs/kernel_modules.html



## 移植

### 重启alloc支持

在编写Rust的no_std程序时，唯一可用的库是rust的core和alloc，其中alloc又需要在开启堆分配的情况下使用，core则是一些最基本的定义，虽然通用但是缺乏许多数据结构。为了在Linux中使用Rust，在较为早期的RFL中，引入了alloc的支持，当时的堆分配只是简单使用内核的`kmalloc`, 但是在后期，alloc库被去掉，内核维护者认为alloc中的堆分配不符合内核的使用方式(Rust程序当堆分配失败时panic，而内核则应该返回错误)。为了解决缺乏堆相关数据结构的问题，相关人员开始在内核中实现一套alloc中的数据结构。

虽然现在已经增加了一些基本的数据结构，但是如果要实现更为复杂的模块，这是数据结构显然还不够用，因此如果我们按照内核现在的路径去添加alloc中对应的数据结构，则需要花费大量的时间，因此我们可以暂时重新使能alloc的支持，当RFL逐渐演化并增加更多的数据结构，我们再替换回来。

为了重新使能alloc，我们需要对内核做相应的修改：

1. 在rust/Makefile中， 我们删除以下的限制

![image-20240908193658601](./assert/image-20240908193658601.png)

2. 在rust/prelude中，我们可以导出alloc中的数据结构，也可以直接使用`use alloc::`。在编译内核时，编译器会抱怨一些函数没有实现，这些函数是堆分配失败时触发的函数，我们需要手动定义，为了简单期间，这里只是简单的panic:

```rust
#![feature(alloc_error_handler)]
/// The Rust allocation error handler.
#[alloc_error_handler]
fn oom(_layout: Layout) -> ! {
    panic!("Out of memory!");
}

/// The Rust allocation error handler.
#[no_mangle]
pub fn __rust_alloc_error_handler(_size: usize, _align: usize) -> ! {
    panic!("Out of memory!");
}

// This symbol is emitted by rustc next to __rust_alloc_error_handler.
// Its value depends on the Zoom={panic,abort} compiler option.
#[no_mangle]
static __rust_alloc_error_handler_should_panic: u8 = 1;
```

现在我们就可以使用alloc中的数据结构来实现内核模块了。

### **[linux-kernel-module-rust](https://github.com/Godones/linux-kernel-module-rust)**

这个项目是早期在Linux上用rust编写内核模块的尝试，但后期暂停并把重心转移到了新的RFL项目上，与当前的RFL不同之处在于这个项目是可以直接编写树外模块并使用rust的包管理设施的。

这个项目的工作流程大致如下:

1. 使用bindgen生成内核头文件的rust绑定，并提供内核函数的封装，这与当前的RFL是相同的。
2. 用户使用封装好的Rust宏来定义模块入口和出口，并作为一个lib进行编译
   1. lib会被编译成静态库，并生成.ko文件
   2. 这个过程与当前的RFL类似，但是当前在RFL中基本只能支持树内Rust模块，并且这些模块都是单个Rust文件构成，只能引用树内的Rust封装
3. 加载内核模块和卸载内核模块

**优势：**

1. 这个项目是根据当前系统的内核头文件生成绑定的，因此其不依赖内核是否支持Rust
2. 因为用户实现的内核模块是一个crate的形式，因此允许我们依赖外部crate

**缺点：**

1. 与当前的RFL相比，这个项目对内核的封装并不完美，并且对Rust的堆分配处理也很简单，这并不符合Linux社区的要求
2. 项目已经年久失修，其支持的内核版本小于5.7(从相关的Issue来看)，Rust版本也是非常久远的1.4*

#### Rebuild

因为我们需要寻找可以使用Rust的Crate包实现内核模块的方法，因此我们尝试将这个工作在比较新的内核版本和最新的Rust工具链上运行。我们需要逐步修复已有的错误：

##### 确认内核版本和rust版本

由于项目支持的kernel和rust版本较老，我们将内核版本更新为之前给WSL编译的一个较新的版本，同时Rust版本也固定到最新的版本。

内核版本:

```
Linux Alien 6.1.21.2-microsoft-standard-WSL2+ #1 SMP Sun Apr  7 10:28:09 CST 2024 x86_64 x86_64 x86_64 GNU/Linux
```

rust版本：

```
rustc 1.82.0-nightly (176e54520 2024-08-04)
```

clang版本

```
 Ubuntu clang version 14.0.0-1ubuntu1.1
```



##### 拉取仓库代码，并尝试运行hello_world例子

修改`Kbuild`的相关命令

```makefile
obj-m := hello_world.o
helloworld-objs := hello_world.o

CARGO ?= cargo
TARGET := x86_64-kernel
TARGET_PROFILE := ./x86_64-kernel.json
BUILD := release

export c_flags

$(src)/target/$(TARGET)/$(BUILD)/libhello_world.a: cargo_will_determine_dependencies
	cd $(src); $(CARGO) build --$(BUILD) -Z build-std=core,alloc --target=$(TARGET_PROFILE)

.PHONY: cargo_will_determine_dependencies

%.o: target/$(TARGET)/$(BUILD)/lib%.a	
	$(LD) -r -o $@ --whole-archive $<

```

**使用自定义的编译目标三元组**

这里最终要的改变是加入了一个设置:

```
"relro-level": "off"
```

如果没有这个设置，最终编译生成的文件会产生一个重定位项:`R_X86_64_GOTPCREL`，新的内核不支持这个重定位项。

网络上有一两个关于这个重定位项的讨论

https://github.com/fishinabarrel/linux-kernel-module-rust/pull/67

https://github.com/rust-lang/rust/issues/57390 

但他们给出的办法在当前的Rust版本上都不可用，添加这个设置的办法也是碰巧从https://users.rust-lang.org/t/how-can-i-enable-plt-in-a-custom-target/57332 这个回答中看到并且**尝试成功**。

这里还重命名了Rust模块的名称，不然后面编译过程会需要一些奇怪名称的文件，同时我们编译release版本而不是debug版本，debug版本产生了大量无用符号。

这里的一个非常重要的变量是`c_flags`, 这个变量是内核编译时所用的c编译标志，在编译Rust的内核模块时，我们会使用clang进行编译，但是内核会将这个flag传递给clang，而clang对这些标志并不会全部支持，这需要我们对这个标志进行处理，我们在下文讨论这个问题。

**处理c_flags中的不支持项**

如果继续编译，可以看到在生成内核文件绑定时发生错误:

```
error: unknown argument: '-mfunction-return=thunk-extern'
error: unknown argument: '-fzero-call-used-regs=used-gpr'
error: unknown argument: '-fconserve-stack'
error: unsupported option '-mrecord-mcount' for target 'x86_64-unknown-linux-gnu'
error: unknown warning option '-Wno-maybe-uninitialized'; did you mean '-Wno-uninitialized'? [-Wunknown-warning-option]
error: unknown warning option '-Wno-alloc-size-larger-than'; did you mean '-Wno-frame-larger-than'? [-Wunknown-warning-option]
error: unknown warning option '-Wimplicit-fallthrough=5'; did you mean '-Wimplicit-fallthrough'? [-Wunknown-warning-option]
```

为了处理这个错误，我们需要在build.rs中手工去掉这些标志:

```rust
kernel_cflags = kernel_cflags.replace("-mfunction-return=thunk-extern", "");
kernel_cflags = kernel_cflags.replace("-fzero-call-used-regs=used-gpr", "");
kernel_cflags = kernel_cflags.replace("-fconserve-stack", "");
kernel_cflags = kernel_cflags.replace("-mrecord-mcount", "");
kernel_cflags = kernel_cflags.replace("-Wno-maybe-uninitialized", "-Wno-uninitialized");
kernel_cflags = kernel_cflags.replace("-Wno-alloc-size-larger-than", "");
kernel_cflags = kernel_cflags.replace("-Wimplicit-fallthrough=5", "-Wimplicit-fallthrough");
```

**处理内核版本过高的问题**

继续编译，报错:

```
thread 'main' panicked at /home/godones/projects/RFL/linux-kernel-module-rust/build.rs:78:9:
  Please update build.rs with the last 5.x version
```

在生成内核头文件绑定时，这个项目对不同版本做了处理，但是对6以上的大版本不支持，因此，我们在build.rs修改，假装我们是一个较低的版本:

```
if major >= 6 {
        major = 5;
        minor = 6;
}
```

再次编译，会发现出现的错误是关于生成的内核绑定的:

```
unsafe { bindings::printk(fmt_str.as_ptr() as _, s.len() as c_int, s.as_ptr()) };
   |                        ^^^^^^ not found in `bindings`
```

```
missing fields `uring_cmd` and `uring_cmd_iopoll` in initializer of `bindings::bindings::file_operations`
```

```
pub fn getrandom(dest: &mut [u8]) -> error::KernelResult<()> {
   |                                      ----------------------- expected `error::Error` because of this
```

对于第一个问题，暂时无法确定为什么这些头文件没有生成`printk`，但是RFL是正常生成的，我们通过在`src/bindgen.rs`中主动声明这个函数并修改一下名称即可解决:

```rust
extern "C" {
    pub fn _printk(fmt: *const core::ffi::c_char, ...) -> core::ffi::c_int;
}
```

对于第二个问题，这两个函数是较新的内核版本引入的，我们在`file_operations.rs` 手动添加这两个函数：

```rust
   	.......
	write_iter: None,
    uring_cmd: None,
    uring_cmd_iopoll: None,
}
```

对于第三个问题，我们直接将`?`操作符改为`unwrap`即可。

**处理文件缺失**

再次编译，出现错误：

```
linux-kernel-module-rust/hello-world/.hello_world.o.cmd: No such file or directory
```

*.o.cmd文件用于内核加速编译，我们可以直接新建一个空文件绕过这个错误

**处理符号缺失**

再次编译，出现错误;

```
ERROR: modpost: "mcount" [/home/godones/projects/RFL/linux-kernel-module-rust/hello-world/hello_world.ko] undefined!
```

这是由于项目在`src/helpers.c`中定义了函数，并使用clang编译了这个文件，在build.rs中，使用clang编译这个文件时，其会传递之前我们提到的`c_flags`标志，其中的`-pg` 标志影响了这个符号的生成，我们手动删除这个标志:

```
builder.remove_flag("-pg");
builder.compile("helpers");
```

见这里的讨论：https://github.com/fishinabarrel/linux-kernel-module-rust/issues/174

**再次处理c_flags中的不支持项**

继续编译，会出现新的错误，提示错误：

```
CC [M]  /home/godones/projects/RFL/linux-kernel-module-rust/hello-world/hello_world.mod.o
clang: error: unknown argument: '-mfunction-return=thunk-extern'
clang: error: unknown argument: '-fzero-call-used-regs=used-gpr'
clang: error: unknown argument: '-fconserve-stack
....

error: unknown warning option '-Wno-maybe-uninitialized'; did you mean '-Wno-uninitialized'? [-Werror,-Wunknown-warning-option]
error: unknown warning option '-Wno-alloc-size-larger-than'; did you mean '-Wno-frame-larger-than'? [-Werror,-Wunknown-warning-option]
error: unknown warning option '-Wimplicit-fallthrough=5'; did you mean '-Wimplicit-fallthrough'? [-Werror,-Wunknown-warning-option]
```

这是由于在编译内核模块时，会生成一个`.mod.c` 文件，这个文件包含了模块的一些元数据，同样，如果我们编译Rust实现的内核模块，这个文件会被clang进行编译，而其参数同样来自`c_flags`。

与在build.rs中修改编译参数不同，这个编译命令是在当前系统的内核模块编译系统中触发的，其所在位置在

```
/lib/modules/$(shell uname -r)/build/scripts/Makefile.modfinal
```

因此，当前我们需要稍微修改一下里面的逻辑，以去掉这些flag：

```makefile
x_flags := -mfunction-return=thunk-extern -fzero-call-used-regs=used-gpr -fconserve-stack -mrecord-mcount -Wno-alloc-size-larger-than -Wno-maybe-uninitialized -Wimplicit-fallthrough=5

quiet_cmd_cc_o_c = CC [M]  $@
      cmd_cc_o_c = $(CC) $(filter-out $(CC_FLAGS_CFI), $(filter-out $(x_flags),$(c_flags))) -c -o $@ $<
```

现在我们只是简单地把不支持的flag删除掉了。

**其它warning**

在编译时可能还会遇到一些warning，因为最新的Rust编译器需要修改一些过时的内容。

比如会给出`cfg()` 的警告https://blog.rust-lang.org/2024/05/06/check-cfg.html，或者类似下面的警告，我们稍加修改即可。

```
use `addr_of_mut!` instead to create a raw pointer
```



**编译成功**

编译成功后，应该就可以加载和卸载Rust编写的内核模块了。

`sudo insmod hello_world.ko`

```
[110959.225617] Hello kernel module!
```

`sudo rmmod hello_world`

```
[110990.781202] My message is on the heap!
[110990.781204] Goodbye kernel module!
```



### 加载domain到内核地址空间

从形式上来看，域与Linux中的内核模块很相似，他们都被允许动态加载和卸载。但又存在一些区别：

1. LKM不支在线升级，必须在模块卸载后重新加载新的模块
2. LKM与内核其它部分是直接通过公开的函数交互，而域只允许通过域接口访问其它域的功能
3. LKM使用的是运行时链接技术，域直接被编译为可独立加载运行的ELF

为了在Linux中引入域隔离的支持，要解决的一个核心问题是允许加载域到内核地址空间。这个过程与LKM是类似的，简单来说就是：

1. 分配一段连续的地址空间
2. 将代码段/数据段等加载到地址空间中
3. 进行必要的重定位工作

在旧的版本中，内核模块通过`vmalloc`系列接口分配可执行内存，在5.8版本中，`__vmalloc` 不再支持`pgprot`参数，这意味着无法指定分配的内存属性。[commit](https://github.com/torvalds/linux/commit/88dca4ca5a93d2c09e5bbc6a62fbfc3af83c4fca)。在较新的版本(~6.10)中, 内核新增了分配可执行内存的接口:

```c
enum execmem_type {
	EXECMEM_DEFAULT,
	EXECMEM_MODULE_TEXT = EXECMEM_DEFAULT,
	EXECMEM_KPROBES,
	EXECMEM_FTRACE,
	EXECMEM_BPF,
	EXECMEM_MODULE_DATA,
	EXECMEM_TYPE_MAX,
};
void *execmem_alloc(enum execmem_type type, size_t size);
void execmem_free(void *ptr);
```

对应的接口位于[execmem.h](https://elixir.bootlin.com/linux/v6.10/source/include/linux/execmem.h#L118)中。 

可以在当前的[内核模块加载函数](https://elixir.bootlin.com/linux/v6.10/source/kernel/module/main.c#L1205)中看到，目前为内核模块分配内存正是使用这个接口。因此我们应该尝试使用这个接口去完成域模块的加载。

#### 分配接口版本变更

这里我们关注几个重要的版本

| kernel version     |  __vmalloc with pgprot   |       module_alloc       |      execmem_alloc       |
| ------------------ | :----------------------: | :----------------------: | :----------------------: |
| 6.1(WSL2)          | :heavy_multiplication_x: |    :heavy_check_mark:    | :heavy_multiplication_x: |
| 6.8(本机linux环境) | :heavy_multiplication_x: |    :heavy_check_mark:    | :heavy_multiplication_x: |
| 6.10(主线RFL环境)  | :heavy_multiplication_x: | :heavy_multiplication_x: |    :heavy_check_mark:    |



## Reference

[编译 linux for rust 并制作 initramfs 最后编写 rust_helloworld 内核驱动 并在 qemu 环境加载](https://blog.csbxd.fun/archives/1699538198511)

[内核模块入门](https://github.com/ljrcore/linuxmooc/blob/master/%E7%B2%BE%E5%BD%A9%E6%96%87%E7%AB%A0/%E6%96%B0%E6%89%8B%E4%B8%8A%E8%B7%AF%EF%BC%9A%E5%86%85%E6%A0%B8%E6%A8%A1%E5%9D%97%E5%85%A5%E9%97%A8.md)

https://www.zhaixue.cc/kernel/kernel-module_sysmvers.html

[Linux在RISC-V平台下的模块实现](https://crab2313.github.io/post/kernel-module/) 描述内核模块的大致加载流程

[学习使用 vmalloc 系列应用编程接口](https://github.com/apachecn/apachecn-linux-zh/blob/master/docs/linux-kernel-prog/09.md#%E5%AD%A6%E4%B9%A0%E4%BD%BF%E7%94%A8-vmalloc-%E7%B3%BB%E5%88%97%E5%BA%94%E7%94%A8%E7%BC%96%E7%A8%8B%E6%8E%A5%E5%8F%A3)

[[内核模块加载流程图 ](https://www.cnblogs.com/pengdonglin137/p/17822352.html)](https://www.cnblogs.com/pengdonglin137/p/17822352.html)
