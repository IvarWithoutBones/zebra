default: run

alias r := run
alias d := debug
alias g := gdb
alias b := build
alias t := test

qemu_extra_args := env_var_or_default("QEMU_EXTRA_ARGS", "")

# Fetch the path to cargo's build artifacts
kernel_image_path := ```
	test -n "${KERNEL_IMAGE-}" && {
		echo "$KERNEL_IMAGE"
		exit 0
	}

	# Environment variables set by cargo
	TARGET_TRIPLE="${TARGET_TRIPLE:-riscv64gc-unknown-none-elf}"
	PROFILE="${PROFILE:-debug}"
	KERNEL_IMAGE="$(cargo metadata --format-version 1 | jq -r '.target_directory')/$TARGET_TRIPLE/$PROFILE/zebra-kernel"

	test -f "$KERNEL_IMAGE" || {
		echo "no kernel image found at '$KERNEL_IMAGE'" >&2
		exit 1
	}

	echo "$KERNEL_IMAGE"
```

# Build the kernel
build *args:
	cargo build {{args}}

# Run all tests
test:
	cargo check
	cargo clippy -- -D warnings
	cargo fmt -- --check
	cargo test
	@echo "tests passed"

# Run the kernel in QEMU
run kernel_path=(kernel_image_path):
	qemu-system-riscv64 \
		-machine virt \
		-cpu rv64 \
		-bios none \
		-smp 1 \
		-m 128M \
		-nographic \
		-serial mon:stdio \
		{{qemu_extra_args}} \
		-kernel {{kernel_path}}

# Run the kernel in QEMU and wait for a GDB connection
@debug kernel_path=(kernel_image_path):
	QEMU_EXTRA_ARGS="-S -gdb tcp::1234 ${QEMU_EXTRA_ARGS:-}" just run {{kernel_path}}

@build-and-debug: build debug

# Connect to a running QEMU instance with GDB
gdb kernel_path=(kernel_image_path):
	rust-gdb --quiet -ex "target remote :1234" {{kernel_path}} || stty onlcr
