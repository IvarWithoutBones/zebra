alias r := run
alias d := debug
alias g := gdb
alias b := build
alias c := check

qemu_extra_args := env_var_or_default("QEMU_EXTRA_ARGS", "")
gdb_port := env_var_or_default("GDB_PORT", "1234")
kernel_image_path := ```
    set -e

	test -z "${KERNEL_IMAGE-}" && {
	    # Environment variables set by cargo
	    TARGET_TRIPLE="${TARGET_TRIPLE:-riscv64gc-unknown-none-elf}"
	    PROFILE="${PROFILE:-debug}"
	    KERNEL_IMAGE="$(cargo metadata --format-version 1 | jq -r ".target_directory")/$TARGET_TRIPLE/$PROFILE/zebra-kernel"
	}

    path="$(realpath "$KERNEL_IMAGE" 2> /dev/null || echo "$KERNEL_IMAGE")"
    test -f "$path" || {
        echo "no kernel image found at '$path'" >&2
        kill $$ # Exit even if we're in a subshell
    }
    echo "$path"
```

# Format all files
format:
    just --unstable --fmt

    @echo "$(tput bold)nixpkgs-fmt ...$(tput sgr0)"
    @nixpkgs-fmt $(find . -type f -name "*.nix")

    cargo fmt

    @echo "formatted all files"

# Run all checks
check:
    just --unstable --check --fmt

    @echo "$(tput bold)nil diagnostics ...$(tput sgr0)"
    @find . -type f -name "*.nix" -print0 | xargs -0 -n1 nil diagnostics

    @echo "$(tput bold)nixpkgs-fmt --check ...$(tput sgr0)"
    @nixpkgs-fmt --check $(find . -type f -name "*.nix")

    cargo fmt -- --check
    cargo check
    cargo clippy -- -D warnings
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
        {{ qemu_extra_args }} \
        -kernel {{ kernel_path }}

# Run the kernel in QEMU and wait for a GDB connection
@debug kernel_path=(kernel_image_path):
    QEMU_EXTRA_ARGS="-S -gdb tcp::{{ gdb_port }} ${QEMU_EXTRA_ARGS:-}" just run {{ kernel_path }}

# Build the kernel
build *args:
    cargo build {{ args }}

@build-and-debug: build debug

# Connect to a running QEMU instance with GDB
gdb kernel_path=(kernel_image_path):
    rust-gdb --quiet -ex "target remote :{{ gdb_port }}" {{ kernel_path }} || stty onlcr
