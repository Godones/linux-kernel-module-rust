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

