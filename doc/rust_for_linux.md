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
make LLVM=1 -j32

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

### 安装riscv gcc工具链

https://blog.csdn.net/qq_43616898/article/details/127911311



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

![image-20240908193658601](./C:/Users/godones/Desktop/研究生培养/assert/image-20240908193658601.png)

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



```
x_flags := -mfunction-return=thunk-extern -fzero-call-used-regs=used-gpr -fconserve-stack -mrecord-mcount -Wno-alloc-size-larger-than -Wno-maybe-uninitialized -Wimplicit-fallthrough=5 -ftrivial-auto-var-init=zero -fsanitize=bounds-strict -mharden-sls=all


cmd_cc_o_c = $(CC) $(filter-out $(CC_FLAGS_CFI) $(CFLAGS_GCOV), $(filter-out $(x_flags),$(c_flags))) -c -o $@ $<
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

#### 6.1版本实现

6.1版本上，阅读内核模块的加载过程，其主要流程如下：

1. 通过`module_alloc` 为内核模块分配一段连续的虚拟地址空间
2. 将各个段加载到内核地址空间
3. 通过`set_memory_x` 将代码段的区间设置为可执行属性



在实现过程中发现，对于`module_alloc` `module_memfree` `set_memory_x` 等函数，内核已经不再导出这些符号，这意味着无法直接在内核模块中使用，为了解决这个问题，需要一些hack。

1. 在编译模块前在内核的符号表中动态查找这几个函数的地址，生成配置文件。由于内核启动后这些地址不会改变，只要在运行前查找到此符号即可
2. 模块使用这些地址转为对应的函数(确保函数定义相同)，并调用

域的私有堆使用两级分配器进行管理，因此tcb模块中需要进行页的分配，这可以使用`vzalloc` 进行分配。

原有的Alien中的域加载过程和在Linux中的相同，为了简化后续的移植，将`loader` 作为一个domain-lib独立出去，同时通过接口完成对外依赖。

由于在linux中目前还无法直接获取域的文件，因此通过直接包含到模块二进制的方式实现。通过简单的尝试，已经可以加载域并且调用域的功能了。

```
81.450101] [LKM] [ INFO] [tcb::domain_loader::creator] Load LogDomain domain, size: 45KB
[17481.450104] [LKM] [ INFO] [loader] copy data to 0x0-0x4000
[17481.450104] [LKM] [ INFO] [loader] copy data to 0x4000-0x5000
[17481.450105] [LKM] [ INFO] [loader] copy data to 0x5000-0x6000
[17481.450105] [LKM] [ INFO] [loader] copy data to 0x6000-0x6a68
[17481.450106] [LKM] [ INFO] [loader] copy data to 0x7000-0x70c0
[17481.450106] [LKM] [ INFO] [loader] copy data to 0x8000-0x9000
[17481.450106] [LKM] [ INFO] [loader] copy data to 0x9000-0xa01c
[17481.450224] [LKM] [ INFO] [loader] set_memory_x range: 0xffffffffc0756000-0xffffffffc075a000
[17481.450224] [LKM] [ INFO] [loader] entry: 0xffffffffc0756150
[17481.450225] [LKM] create domain database for domain_id: 1
[17481.450233] [0][Domain:1] Logger init
[17481.450234] [LKM] [ERROR] [tcb::domain_helper::sheap] <SharedHeap> alloc size: 18, ptr: 0xffff9c9e8178b7a0
[17481.450610] [0][Domain:1] [ERROR] print using logger
[17481.450611] [LKM] free_domain_resource for domain_id: 1
[17481.450612] [LKM] domain_id: 0, count: 1
[17481.450612] [LKM] <checkout_shared_data> shared heap size: 1
[17481.450612] [LKM] <free_domain_shared_data> shared heap size: 1
[17481.450613] [LKM] <free_domain_shared_data> for domain_id: 1
[17481.450613] [LKM] domain has 0 data
[17481.450613] [LKM] free_shared is Free, free 0 data
[17481.450614] [LKM] [Domain: 1] free DomainDataMap resource
[17481.450845] [LKM] [ WARN] [rref::rvec] <drop> for RRefVec
[17481.450845] [LKM] [ WARN] [rref::rref] <drop> for RRef 0xffff9c9e8178b7a0
[17481.450846] [LKM] [ WARN] [rref::rref] <custom_drop> for RRef 0xffff9c9e8178b7a0
[17481.450846] [LKM] [ WARN] [rref] default for u8
[17481.450847] [LKM] [ERROR] [tcb::domain_helper::sheap] <SharedHeap> dealloc: 0xffff9c9e8178b7a0
[17481.451171] [LKM] [ INFO] [loader] drop domain loader [logger]
[17481.451179] [LKM] Dropping VirtArea: ffffffffc0756000
[17481.451179] [LKM] [ INFO] [loader] drop domain loader [gnull]
[17481.451180] [LKM] Dropping VirtArea: ffffffffc0752000
[17485.859590] [LKM] My message is on the heap!
[17485.859593] [LKM] Goodbye kernel module!
```

## 实现

这部分描述引入域的支持的实现。

### 6.6版本支持

在完成6.1版本的移植后，域的基本支持已经具备了。但后期我们需要将已有的Rust实现的驱动改造成域的形式，而这些驱动在RFL的仓库中，需要一些较高的linux版本。因为null block device的驱动比较时候改造和测试，这里也是选择了其支持的较早的版本。

6.1版本到6.6版本的改动并不是很大，只需要调整一些内核接口变化即可。

因为null block device 使用了大量RFL仓库的实现，因此这里也是把之前使用的项目和RFL仓库的代码进行了合并。

### 内核同步原语

#### per-cpu变量和RCU

[Linux driver example for Per CPU Variable](https://embeddedguruji.blogspot.com/2019/05/linux-driver-example-for-per-cpu.html)

[KernelPerCPUVariable](https://github.com/ANSANJAY/KernelPerCPUVariable)

linux提供了分配和访问per-cpu变量的接口，我们只需要对这些接口进行封装并提供给Rust使用，但需要注意的是，C接口是一个宏，它需要传入变量的类型，这无法通过FFI完成，因此我们特化了longlong 的实现，对应到Rust的u64，因为per-cpu变量被我们后续用做计数器。

```rust
#[link_name = "rust_helper_num_online_cpus"]
pub(crate) fn num_online_cpus() -> core::ffi::c_uint;
#[link_name = "rust_helper_alloc_percpu_longlong"]
pub(crate) fn alloc_percpu_longlong() -> *mut core::ffi::c_longlong;
#[link_name = "rust_helper_free_percpu_longlong"]
pub(crate) fn free_percpu_longlong(p: *mut core::ffi::c_longlong);
#[link_name = "rust_helper_get_cpu"]
pub(crate) fn get_cpu() -> core::ffi::c_int;
#[link_name = "rust_helper_put_cpu"]
pub(crate) fn put_cpu();
```

在无状态域的热升级中，我们使用RCU方法。我们也不需要重新实现RCU，与Per-cpu变量类似，RCU的接口也是一些宏，需要做额外的处理。

```c
struct rcudata {
    void *a;
};
void * rust_helper_rcu_dereference(struct rcudata *p) { return rcu_dereference(p->a); }
void rust_helper_rcu_assign_pointer(struct rcudata *p, void *v) { rcu_assign_pointer(p->a, v); }
```

在封装C接口时，我们需要用一个结构体包裹真正的数据，否则在RCU宏展开后，其赋值会作用在临时变量上。

#### 互斥锁/自旋锁

在有状态域的实现中，需要用锁来保证域更新期间只有更新者进入。我们利用自旋锁来完成这个工作，这两个锁的实现在RFL项目中已经包含，因此我们可以直接使用。但要注意，这里使用了自引用数据结构，需要Pin数据结构来做保证。

### 域的热更新实现

这部分的实现在第二层域代理中实现，与在Alien中的实现说类似的，只是把对应的数据结构换成了linux的。当前我们只实现了两个简单的域来做演示，一个是`LogDomain`, 另一个是`EmptyDeviceDomain`, 我们把`LogDomain`视为无状态域，`EmptyDeviceDomain`作为有状态域来检查域更新机制实现的正确性。

### 测试域功能

当前我们实现的两个域并不包含什么实际功能，想要应用程序使用到域的功能，一个做法是增加新的系统调用，通过系统调用访问域的功能，但是在内核模块中增加系统调用已经无法实施，这里我们选择另一个做法来测试域的功能。通过内核模块，我们创建两个虚拟的文件，并在文件的读写回调函数中调用域的功能。在用户态，通过读写这两个虚拟文件，就可以触发对域功能的访问，进而可以对域的热更新机制进行测试。

**向内核添加系统调用**

https://chenhaotian.top/linux/linux-kernel-add-syscall/index.html

https://www.cnblogs.com/wangzahngjun/p/4992045.html

[Linux 系统调用（二）——使用内核模块添加系统调用（无需编译内核）](https://blog.csdn.net/qq_34258344/article/details/103228607)

[[Hooking syscall by modifying sys_call_table does not work](https://stackoverflow.com/questions/78599971/hooking-syscall-by-modifying-sys-call-table-does-not-work)](https://stackoverflow.com/questions/78599971/hooking-syscall-by-modifying-sys-call-table-does-not-work)



## 驱动移植

### 如何对接linux的接口和域的接口

在Linux中，内核可加载模块的常用做法是通过向内核注入相关设备驱动程序或者文件系统的回调函数来实现扩展内核的功能。同时，这些模块可以使用内核导出的符号，向内核申请资源或者释放资源。内核模块的加载由相关的系统调用完成，加载程序通过调用LKM定义的`init`函数， 而LKM在`init`函数中向内核注册功能，在模块卸载时，通过调用LKM的`exit`把之前LKM注册的功能删除。

为了将一个LKM改造成域的实现，需要对LKM与内核的交互方式进行分析。首先，一个域是只依赖domain-lib或者其它域的以及其它不包含unsafe代码的依赖。在Alien中，一个最小的mini-core作为TCB提供最核心的功能，而domain-lib通过对TCB提供的功能完成所有unsafe代码的封装和检查(Page,TaskContext)，因此系统中的unsafe代码只会来自TCB和domain-lib。

当将域的设计应用到Linux kernel时，因为整个kernel是作为一个整体存在的，并不存在类似Alien中的mini-core。当前在kernel中使用Rust重写模块处在初步阶段，我们也无法做到脱离kernel的C代码。为此，我们需要把kernel整体作为一个TCB来看待。此时kernel导出的符号就等价于TCB提供的核心功能。

> 此时我们把kernel当作可信基，意味着我们相信kernel提供的功能

在domain-lib中，我们会把这些接口进行安全封装，将像当前RFL项目所做的那样。

这里存在两个问题：

1. 许多接口用于向kernel申请和释放资源(除了堆之外的资源)
2. 扩展内核功能是通过注册回调函数来完成的

在Alien中，TCB提供的资源是用于构建私有堆的物理页，这些资源可以在域被卸载时安全地释放掉。当Linux kernel成为TCB，其还提供了许多系统资源，这些系统资源需要手动地进行回收，这意味着如果域使用了这些资源，那当域卸载时，并不是简单地就能把域的资源全部回收掉。

如果域也通过注册回调函数来扩展kernel的功能，那么当域被卸载时，这些回调函数也需要被正确地删除。在Alien中，整个系统被划分为多个域，这些域并不是向其它域注入回调函数来扩展域的功能的。同时如果域使用注册回调函数的话，那域的接口就被简化了，因为这个时候Linux kernel不是通过调用域的接口完成功能而是域内部的回调函数。这种方法会导致无法对域进行热更新操作，因为调用域提供的功能时并不是从域的接口进入，域代理无法感知域是否正在被使用，就不能正确同步。

解决第一个问题的方法是在域接口上增加一个`exit` ，在域被替换回收资源时调用。因为domain-lib对kernel资源已经做了抽象，域内部只需要调用`Drop`就能释放这些资源。

> 通过逐渐将更多kernel中的功能变成域实现，我们可以逐步过渡到更好的实现上

为了解决第二个问题，我们需要禁止域去注册回调函数，而是**通过创建内核功能到域功能的中介来间接地使用域功能**，但是这种方式需要对内核的功能逐个做封装，并且需要仔细设计域的接口形式。在域的接口上，我们只能使用`RRef<T>` 相应的共享堆上的数据结构来进行通信，而内核中的大多数数据结构包含了裸指针，因此它们之间的对应关系也需要进行转换。

到现在为止，可以看到这些域的实现和当前用Rust实现的内核模块是很相似的，但它们又存在一些区别:

|          | LKM                                                 | Domain                                  |
| -------- | --------------------------------------------------- | --------------------------------------- |
| 编译运行 | 独立编译，kernel对其引用的符号进行重定位处理        | 独立编译，不需要对符号重定位处理(trait) |
| 内核资源 | 手动分配，手动回收                                  | 自动分配回收和释放(资源抽象/Drop)       |
| 热更新   | 不支持，kernel没有提供相关的状态管理和同步措施      | 支持                                    |
| safe     | 不强制safe code实现， LKM之间没有界限(符号互相引用) | 强制safe code实现， 域之间通过接口      |

原理上来说，Linux的内核模块在C语言也可以实现域的所有性质，但一些语言特性可能无法达到。

**接口限定**

把LKM使用的所有kernel导出的符号集中在一个结构体中 等价于 kernel作为TCB提供功能(trait)，这样可以限定LKM所能使用的kernel接口，使得LKM不会访问kernel的任意函数。

> 在C语言需要额外的工具进行检查，因为C语言不限制对指针的任意转换。而当使用safe的Rust语言实现的时候，在编译时就可以阻止这种行为

可以将LKM提供的接口集中在一个结构体中，等价于域的接口。这样不管是kernel使用LKM还是其它LKM依赖都可以明确LKM提供的功能。

> 在当前的kernel形态下，这就需要在LKM提供的接口和内核回调函数之间实现中介层

**私有堆/共享堆/自定义堆**

LKM也可以应用私有堆和共享堆，这个是显然的。但应用共享堆的前提是LKM实现了功能限定。

> LKM需要工具检查是否违反规定。Rust可以在语言层面定义规则

**内存隔离**

有了上述两个性质还不够使得使用C实现的LKM就具备了域的隔离性质，因为在实现域时还有一个重要的保证是域只能使用安全的Rust实现(内存安全)。在C中，需要使用工具检查这个性质

**故障隔离**

C代码中不提供unwind的支持，无法使用语言方法来进行故障隔离。

**热升级**

通过为LKM提供状态保存恢复和域更新同步机制，LKM也可以实现热升级。



### null block device 改造

通过将已有的null block device驱动改造为域实现，可以对比C实现/非域Rust实现/域实现之间的性能差距。同时测试故障恢复与热更新的影响。

在块设备驱动中，核心的有两个数据结构:

```rust
/// A wrapper for the C `struct blk_mq_tag_set`
pub struct TagSet<T: Operations> {
    inner: UnsafeCell<bindings::blk_mq_tag_set>,
    _p: PhantomData<T>,
}
```

```rust
/// A generic block device
pub struct GenDisk<T: Operations> {
    _tagset: Arc<TagSet<T>>,
    gendisk: *mut bindings::gendisk,
}
```

其中`TagSet` 是注册回调函数的结构体，在它的初始化过程中，会初始化一些参数，并向kernel分配数据结构:

```rust
inner.ops = unsafe { OperationsVtable::<T>::build() };
inner.nr_hw_queues = nr_hw_queues;
inner.timeout = 0; // 0 means default which is 30 * HZ in C
inner.numa_node = bindings::NUMA_NO_NODE;
inner.queue_depth = num_tags;
inner.cmd_size = core::mem::size_of::<T::RequestData>().try_into()?;
inner.flags = bindings::BLK_MQ_F_SHOULD_MERGE;
inner.driver_data = tagset_data.into_foreign() as _;
inner.nr_maps = num_maps;

// SAFETY: `inner` points to valid and initialised memory.
let ret = unsafe { bindings::blk_mq_alloc_tag_set(inner) };
```

这些过程应该由驱动内部实现，中间对象不需要再重复。

`TagSet`会 构建`blk_mq_ops`, 这是回调函数的来源，中间对象需要处理这些对象。

```rust
 const VTABLE: bindings::blk_mq_ops = bindings::blk_mq_ops {
    queue_rq: Some(Self::queue_rq_callback),
    queue_rqs: None,
    commit_rqs: Some(Self::commit_rqs_callback),
    get_budget: None,
    put_budget: None,
    set_rq_budget_token: None,
    get_rq_budget_token: None,
    timeout: None,
    poll: if T::HAS_POLL {
        Some(Self::poll_callback)
    } else {
        None
    },
    complete: Some(Self::complete_callback),
    init_hctx: Some(Self::init_hctx_callback),
    exit_hctx: Some(Self::exit_hctx_callback),
    init_request: Some(Self::init_request_callback),
    exit_request: Some(Self::exit_request_callback),
    cleanup_rq: None,
    busy: None,
    map_queues: if T::HAS_MAP_QUEUES {
        Some(Self::map_queues_callback)
    } else {
        None
    },
    show_rq: None,
};
```

这些回调函数的签名基本都是裸指针:

```rust
unsafe extern "C" fn queue_rq_callback(
        hctx: *mut bindings::blk_mq_hw_ctx,
        bd: *const bindings::blk_mq_queue_data,
    ) -> bindings::blk_status_t;
unsafe extern "C" fn commit_rqs_callback(hctx: *mut bindings::blk_mq_hw_ctx);
unsafe extern "C" fn complete_callback(rq: *mut bindings::request);
unsafe extern "C" fn poll_callback(
    hctx: *mut bindings::blk_mq_hw_ctx,
    _iob: *mut bindings::io_comp_batch,
) -> core::ffi::c_int 
```

也就是说，在kernel调用这些回调函数时，应该从域的接口处进入，所有域的接口需要被设计为传递这些参数：

- 依然保持驱动域分配和初始化`TagSet<T>` 和 `GenDisk<T>` 结构，除了两个结构里面的回调函数(以及可能的kernel资源)

> 在域接口上不允许使用裸指针，所以改为返回指针的值，因为只有TCB理解这个值的含义，对于其它域来说，这个值是无意义的

- 域需要实现回调函数规定的功能
- kernel将指针转换为原始结构，并使用中介对象将域接口转为回调函数

 <font color = red>**注意**</font>

在`TagSet<T>` 这个初始化过程中，需要先填充`ops`才能调用`blk_mq_alloc_tag_set`去分配相应的硬件队列，以及软件队列，因为kernel在分配这些队列的时候，又会通过其中一些回调函数让驱动完成一些工作。所以在驱动侧不应该去分配硬件队列，应该由中间对象来进行分配。

在`blk_mq_alloc_tag_set`期间，kernel会调用`init_request_callback` 回调函数初始化对应数量的`Request`

```
[39658.998522] rust_kernel: before __blk_mq_alloc_disk
[39658.998537] rust_kernel: init_hctx_callback began, hctx: 0
[39658.998538] rust_kernel: init_hctx_callback ended
[39658.998539] rust_kernel: init_request_callback began, call count: 256, 0xffff8f4b71c10600
[39658.998539] rust_kernel: init_request_callback ended
[39658.998579] rust_kernel: after __blk_mq_alloc_disk
```

在`__blk_mq_alloc_disk`期间，kernel会调用`init_hctx_callback` 初始化硬件队列，并且初始一个特殊的`request`。

在`device_add_disk` 期间,kernel会调用`init_request_callback` 初始化另外的对应数量的`Request`

在`TagSet<T>` 和 `GenDisk<T>` 释放期间，会执行相反的过程回收这些资源。

kernel对驱动域的调用过程:

1. 创建和初始化域
   1. 域创建`TagSet<T> ` ,`GenDisk<T>`数据结构，但只是填充初始的配置信息，不调用`blk_mq_alloc_tag_set` ，而是让kernel分配数据
2. shim对象从域获取tagset的指针`domain.tag_set_with_queue_data()`, 创建tagset的ops，并重定向到域的实现
3. shim对象调用`blk_mq_alloc_tag_set` 根据tagset分配`Request` ，如果失败，则让驱动域回收资源
4. shim对象从驱动域拿到queue_data的指针，与tagset的指针一起分配gendisk结构。如果失败，则驱动域回收资源。如果成功，shim对象得到gendisk指针
5. shim对象设置驱动域的gendisk指针，(`domain.set_gen_disk()`), 驱动域紧接着设置相应的配置信息
6. shim对象调用 `device_add_disk(gen_disk)`  向内核注册该设备，kernel在此期间再一次调用回调函数(gen_disk->tagset->ops)分配`Request`
7. 运行期间对gendisk和tagset的回调全部被重定向到域接口上
8. 当域被卸载，shim对象调用`del_gendisk(gen_disk)` 释放gen_disk数据(Request)，调用`blk_mq_free_tag_set` 释放释放tagset数据(Request), kernel在此期间再一次调用回调函数(已经被重定向到域接口上)
9. shim对象让域释放从kernel拿到的资源`domain.exit()`, 释放域占用的资源



https://linux-kernel-labs-zh.xyz/labs/block_device_drivers.html

### linux kernel的故障隔离

为了在linux中启用故障隔离，引入了Rust的unwind机制，在测试过程中，发现内核会崩溃：

```
[  493.849106] BUG: TASK stack guard page was hit at 00000000635c0575 (stack is 000000000a190a4c..00000000dc307f6a)
[  493.849112] stack guard page: 0000 [#1] PREEMPT SMP NOPTI
[  493.849116] CPU: 8 PID: 4943 Comm: insmod Tainted: G           OE      6.6.0-microsoft-standard-WSL2+ #4
[  493.849120] RIP: 0010:_ZN5gimli4read3cfi24UnwindTable$LT$R$C$S$GT$8next_row17h25548a9c73cd9ffaE+0x4d/0x2270 [hello_world]
[  493.849144] Code: 00 00 48 8b 56 58 48 69 c9 38 04 00 00 48 89 94 08 00 fc ff ff c6 46 69 00 48 8b 56 30 48 85 d2 0f 84 94 1b 00 00 48 8d 46 28 <48> 89 84 24 98 00 00 00 44 0f b6 76 38 0f b6 6e 39 0f b6 5e 3a 4c
[  493.849146] RSP: 0018:ffffa2d614093aa0 EFLAGS: 00010202
[  493.849148] RAX: ffffa2d614094508 RBX: 0000000000000008 RCX: 0000000000000438
[  493.849149] RDX: 0000000000000007 RSI: ffffa2d6140944e0 RDI: ffffa2d614094550
[  493.849151] RBP: ffffa2d614094ea8 R08: fffffffffffffff8 R09: 0000000000000000
[  493.849152] R10: 00ffff9681c78201 R11: ffffffffc05fde04 R12: ffffa2d6140944e0
[  493.849153] R13: ffffa2d614095090 R14: ffffa2d614094550 R15: ffffa2d614095070
[  493.849154] FS:  0000726e698ba080(0000) GS:ffff9681c7800000(0000) knlGS:0000000000000000
[  493.849156] CS:  0010 DS: 0000 ES: 0000 CR0: 0000000080050033
[  493.849157] CR2: ffffa2d614093a98 CR3: 000000011e0ae000 CR4: 0000000000350ee0
[  493.849160] Call Trace:
[  493.849162] WARNING: kernel stack frame pointer at 00000000389b6155 in insmod:4943 has bad value 00000000b73c08b7
[  493.849164] unwind stack type:0 next_sp:0000000000000000 mask:0x2 graph_idx:0
```

从捕捉到的信息来看，内核崩溃是因为task的内核栈溢出。通过查询内核源代码得知，kernel给task的内核栈大小为4*PAGE_SIZE, 由于故障期间需要对内核栈进行展开以及Rust通常分配较大的函数栈导致了栈溢出。





**解决这个问题的方法是修改内核源码，增大内核栈大小到8*PAGE_SIZE**。

https://stackoverflow.com/questions/65121145/how-to-test-validate-the-vmalloc-guard-page-is-working-in-linux

https://stackoverflow.com/questions/27478788/debug-stack-overruns-in-kernel-modules





### 文件系统

tarfs/ext2

```
创建空的磁盘镜像文件
dd if=/dev/zero of=ext_image bs=1M count=512
使用 losetup将磁盘镜像文件虚拟成块设备
sudo losetup /dev/loop1 ./ext_image
卸载
sudo umount
sudo losetup -d /dev/loop1
```



https://www.cnblogs.com/wuchanming/p/4690474.html

https://linux-kernel-labs.github.io/refs/heads/master/labs/filesystems_part1.html

## 性能测试

### null block

机器: 

- WSL2 ubuntu24.04 linux6.6
- AMD Ryzen 9 7945HX()
- 16GB

配置：

driver:

1. block_size: 4KB
2. completion_nsec: 0
3. irqmode: 0(IRQ_NONE)
4. queue_mode:2 (MQ)
5. hw_queue_depth: 256
6. memory_backed:1
7. queue scheduler:none

fio:

1. run  30 second
2. IO engine: PSYNC





Thread=1 cnull

|            | randread              | randrw                | randwrite             | read                | readwrite            | write                       |
| ---------- | --------------------- | --------------------- | --------------------- | ------------------- | -------------------- | --------------------------- |
| 4k         | 234k+233k+229k = 232k | 91+98+85= 91          | 168k+188k+193k = 183k | 428k+454k+465k=449k | 180k+174k+175k = 176 | 213k+215k+215k = 214k       |
| 32k        | 61+61.3+60.2 = 60.8   | 19.4+19.5+19.3 = 19.4 | 34.3+33.9+33.7= 33.97 | 72+73+71 = 72       | 32+31.3+30.7 = 31.33 | 34.9 + 33.9 + 34.5 =  34.43 |
| 256k       |                       |                       |                       |                     |                      |                             |
| 1024k=1M   |                       |                       |                       |                     |                      |                             |
| 16384k=16M |                       |                       |                       |                     |                      |                             |

Thread=1 rnull(LKM)

|            | randread                        | randrw                                  | randwrite                    | read                           | readwrite                        | write                        |
| ---------- | ------------------------------- | --------------------------------------- | ---------------------------- | ------------------------------ | -------------------------------- | ---------------------------- |
| 4k         | 209k+210k+204k = 207k(-0.107)   | 83 +85+84 =84 (-0.07,-0.07)             | 191k+197k+186k = 192k(0.049) | 425k+455k+449k = 443k (-0.013) | 179+168+172= 173 (-0.017,-0.017) | 208k+220k+210k = 213(-0.004) |
| 32k        | 61.0k+60.2+60.5 = 60.6(-0.0038) | 19.4+19.6+19.4 = (19.47,19.47) (0.0034) | 34.1+34.5+34.0=34.2 (0.0067) | 72+71+72.5 = 71.8(-0.003)      | 30.6+30.1+30.0 = 30.23(-0.035)   | 36.9 + 36.7+ 36 = 36.5(0.06) |
| 256k       |                                 |                                         |                              |                                |                                  |                              |
| 1024k=1M   |                                 |                                         |                              |                                |                                  |                              |
| 16384k=16M |                                 |                                         |                              |                                |                                  |                              |

Thread=1 rnull(Domain)

|      | randread                             | randrw                               | randwrite                               | read                                | readwrite                               | write                                |
| ---- | ------------------------------------ | ------------------------------------ | --------------------------------------- | ----------------------------------- | --------------------------------------- | ------------------------------------ |
| 4k   | 206k+199k+201k=  202(-0.129)(-0.024) | 81.7+79.8+81.7= 81.1(-0.108)(-0.034) | 195k+191k+192k = 192.6 (0.05)(0.003125) | 452k+447k+448k = 449 (0) (0.013)    | 177k+174k+169k= 173(-0.017,-0.017)(0,0) | 208k+216k+207k = 210(-0.017)(-0.014) |
| 32k  | 48.3 + 48.1+46.7 =                   | 17.8 + 17.5+17.9= 17.7(-0.08)(-0.08) | 32.2+31.9+32.6 =32.2(-0.05)(-0.058)     | 60.4+60.3+60.1 =60.3(-0.162)(-0.16) | 30.4+29.5+28.6 = 29.5(-0.058)(-0.024)   | 34.0+33.6+34.1 = 33.9(-0.015)(-0.07) |





Thread=4 cnull

|      | randread                 | randrw                | randwrite             | read                       | readwrite             | write                  |
| ---- | ------------------------ | --------------------- | --------------------- | -------------------------- | --------------------- | ---------------------- |
| 4k   | 1447k+1483k+1414k = 1448 | 427k+424k+416k = 422  | 535k+547k+565k=549    | 1812k + 1812k+1812k = 1812 | 895k+889k+869k =884   | 755k+762k+759k =  759k |
| 32k  | 338k+343k+339k = 340     | 69.2+66.9+67.4 = 67.8 | 77.9+82.2+82.4 = 80.8 | 170k+172k+169k = 170       | 149k+150k+143k =  147 | 119k+120k+121k = 120   |

Thread=4 rnull(LKM)

|      | randread                        | randrw                        | randwrite                    | read                            | readwrite               | write                        |
| ---- | ------------------------------- | ----------------------------- | ---------------------------- | ------------------------------- | ----------------------- | ---------------------------- |
| 4k   | 1265k+1253k+1244k= 1254(-0.133) | 377k+393k+388k = 386(-0.085)  | 530k+532k+540k = 534(-0.027) | 1731k+1726k+1744k = 1733(-0.04) | 872k+900k+879k = 884(0) | 725k+755k+735k = 738(-0.027) |
| 32k  | 322+317+328 = 322(-0.05)        | 66.9+66.5+66.8 = 66.7(-0.015) | 76.5+86.4+82.4 = 81.8(0.012) | 165k+168k+166k = 166(-0.02)     | 144+142+148=145(-0.016) | 115k+116k+118k=116(-0.03)    |

Thread=4 rnull(Domain)

|      | randread        | randrw | randwrite | read              | readwrite | write          |
| ---- | --------------- | ------ | --------- | ----------------- | --------- | -------------- |
| 4k   | 1266k1241k1163k |        |           | 1591k+1594k+1580k |           | 736k+736k+725k |
| 32k  |                 |        |           |                   |           |                |





Thread=4 

|           | Domain                            | C                       |
| --------- | --------------------------------- | ----------------------- |
| write     | 883k+898k+911k = 897(0.012)       | 896k+880k+881k = 886k   |
| read      | 1933k+1914k+1869k = 1,905(-0.012) | 1955k+1896k+1933k= 1928 |
| write 32k | 138k+138k+138k = 138(0.007)       | 137k+138k+137k = 137    |
| read 32k  | 200k+198k+193k = 197(-0.024)      | 201k+200k+204k = 202    |



- 从实验的整体结果来看，C/LKM/Domain的性能差异并不是很大，LKM和C的差异平均在5%以内，最大的差距是10.5%(randread,4k), 而Domain和LKM的性能差距非常小。
- 域的实现并没有导致原有的LKM性能下降，一方面是因为域实现并没有引入额外的硬件特征，只是多了几层函数调用，另一方面，用户程序发出的系统调用在Linux kernel中经过深层次的处理，而域在这个调用路径上只占据了非常小的位置。
- 我们猜测当kernel中的大多数组件变成域后，性能会有一定的下降，但这个下降处于可接受的水平



## Reference

[编译 linux for rust 并制作 initramfs 最后编写 rust_helloworld 内核驱动 并在 qemu 环境加载](https://blog.csbxd.fun/archives/1699538198511)

[内核模块入门](https://github.com/ljrcore/linuxmooc/blob/master/%E7%B2%BE%E5%BD%A9%E6%96%87%E7%AB%A0/%E6%96%B0%E6%89%8B%E4%B8%8A%E8%B7%AF%EF%BC%9A%E5%86%85%E6%A0%B8%E6%A8%A1%E5%9D%97%E5%85%A5%E9%97%A8.md)

https://www.zhaixue.cc/kernel/kernel-module_sysmvers.html

[Linux在RISC-V平台下的模块实现](https://crab2313.github.io/post/kernel-module/) 描述内核模块的大致加载流程

[学习使用 vmalloc 系列应用编程接口](https://github.com/apachecn/apachecn-linux-zh/blob/master/docs/linux-kernel-prog/09.md#%E5%AD%A6%E4%B9%A0%E4%BD%BF%E7%94%A8-vmalloc-%E7%B3%BB%E5%88%97%E5%BA%94%E7%94%A8%E7%BC%96%E7%A8%8B%E6%8E%A5%E5%8F%A3)

[内核模块加载流程图](https://www.cnblogs.com/pengdonglin137/p/17822352.html)

[WSL升级内核版本](https://blog.csdn.net/fengshantao/article/details/139511245)

[通过内核符号地址调用未导出的函数](https://heapdump.cn/article/1569179)

[linux内存分配接口](https://juejin.cn/post/7369488229336170505#heading-61)



[Linux Kernel Rust Modules](https://tomcat0x42.me/linux/rust/2023/04/07/linux-kernel-rust-modules.html)

[Rust Kernel Programming](https://coderjoshdk.github.io/posts/Rust-Kernel-Programming.html#Everything_you_might_need) Rust for linux 案例

[查找rust支持需要的配置 ](https://codentium.com/building-a-linux-kernel-with-rust-support-on-gentoo/) 自定义内核编译选项

修改wsl2内核 https://enita.cn/2023/0731/bcd47a5aace1/ 

wsl2 滚动发行版 **[WSL2-Linux-Kernel-Rolling](https://github.com/Nevuly/WSL2-Linux-Kernel-Rolling)**



[scull_device_driver ](https://github.com/CriMilanese/scull_device_driver)杂项设备，模拟的是字符类设备

**[linux-kernel-module-rust](https://github.com/lizhuohua/linux-kernel-module-rust)** 早期内核模块的尝试，包含一些简单的示例和论文

FIO使用：https://help.aliyun.com/zh/ecs/user-guide/test-the-performance-of-block-storage-devices

[在 Ubuntu 22.04.3 上构建自定义内核](https://cuefe.com/12/)

[超级用户指南：轻松升级你的Ubuntu Linux内核版本](https://blog.csdn.net/Long_xu/article/details/126710992)
