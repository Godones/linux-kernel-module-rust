export KDIR ?= /lib/modules/$(shell uname -r)/build
module-name := rofs
CLANG ?= clang
ifeq ($(origin CC),default)
CC := ${CLANG}
endif

all:
	touch ./.$(module-name).o.cmd
	$(MAKE) -C $(KDIR) M=$(CURDIR) CC=$(CC) CONFIG_CC_IS_CLANG=y

clean:
	$(MAKE) -C $(KDIR) M=$(CURDIR) CC=$(CC) clean
	-rm ../../target/x86_64-kernel -dr

rebuild: clean all
	@echo "rebuild done"