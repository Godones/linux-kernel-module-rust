export KDIR ?= /lib/modules/$(shell uname -r)/build
module-name := tcb
CLANG ?= clang
ifeq ($(origin CC),default)
CC := ${CLANG}
endif

all:
	touch ./.$(module-name).o.cmd
	$(MAKE) -C $(KDIR) M=$(CURDIR) CC=$(CC) CONFIG_CC_IS_CLANG=y

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR) CC=$(CC) clean
	cargo clean
