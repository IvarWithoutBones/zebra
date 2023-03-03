# vim: set ft=make :

target_triple := env_var_or_default("CARGO_BUILD_TARGET", "riscv64gc-unknown-none-elf")
profile := env_var_or_default("PROFILE", "debug")
qemu_extra_args := env_var_or_default("QEMU_EXTRA_ARGS", "")

alias q := qemu

default: qemu

kernel-image-path:
	@echo `cargo metadata --format-version 1 | jq -r '.target_directory'`/{{target_triple}}/{{profile}}/zebra-kernel

qemu kernel_path=`just kernel-image-path`:
	qemu-system-riscv64 \
		-machine virt \
		-cpu rv64 \
		-bios none \
		-smp 2 \
		-m 128M \
		-nographic \
		-serial mon:stdio \
		-kernel {{kernel_path}} \
		{{qemu_extra_args}}

@debug kernel_path="":
	QEMU_EXTRA_ARGS="-S -s ${QEMU_EXTRA_ARGS:-}" just qemu {{kernel_path}}

@gdb kernel_path=`just kernel-image-path`:
	rust-gdb --quiet -ex "target remote :1234" {{kernel_path}}
