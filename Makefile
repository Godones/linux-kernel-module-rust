
runtime:
	make -C tcb rebuild

domains:
	cargo domain build-all -a x86_64

run: domains runtime
	sudo insmod tcb/tcb.ko
	sudo dmesg | tail -n 10

.PHONY: runtime run domains