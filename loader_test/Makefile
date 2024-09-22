export KDIR ?= /lib/modules/$(shell uname -r)/build
module-name := loader_test
CLANG ?= clang
ifeq ($(origin CC),default)
CC := ${CLANG}
endif

CARGO ?= cargo
TARGET := x86_64-kernel
TARGET_PROFILE := ../x86_64-kernel.json
BUILD := release


all:
	touch ./.$(module-name).o.cmd
	$(MAKE) -C $(KDIR) M=$(CURDIR) CC=$(CC) CONFIG_CC_IS_CLANG=y

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR) CC=$(CC) clean
	cargo clean

rebuild: clean all
	@echo "Rebuild done"