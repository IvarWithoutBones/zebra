# Disable generation of compressed instructions
.option norvc

.section .text.init
.global _start
_start:
	# Stop the assembler from assuming that `gp` is already initialized
    .option push
    .option norelax
	la gp, _global_pointer
    .option pop

	# Any hardware threads (harts) other than hart 0 should park until multithreading is implemented
	csrr t0, mhartid
	bnez t0, park_hart

	# Clear the BSS section to avoid UB
	la a0, _bss_start
	la a1, _bss_end
clear_bss_loop:
	sd zero, (a0)
	addi a0, a0, 8
	bltu a0, a1, clear_bss_loop

    # Initialize the stack, assuming one hart
	la sp, _stack_end

    # Set a Machine trap handler. This should never be called as we delegate all traps to user mode.
    # In the odd case it is we will panic the kernel.
    la t0, machine_trap_vector
    csrw mtvec, t0

    # Delegate all traps to Supervisor
    li t0, 0xffffffffffffff
    csrw medeleg, t0
    csrw mideleg, t0

    # Set the Machine Previous Privilege mode to Supervisor, this will apply once we call `mret`
    li t0, 1 << 11
	csrw mstatus, t0

    # Set the Physical Memory Protection to allow Supervisor to access all memory
    li t0, 0xf
    csrw pmpcfg0, t0

	# Temporarily disable paging, will be enabled by the kernel once its ready
	csrw satp, zero

	# Set up the jump to the kernels entry point
	la t0, kernel_main
	csrw mepc, t0

	# Place to continue execution after the kernel has finished (should never be reached)
	la ra, park_hart

	# Enter supervisor mode and jump to the kernel
	mret

.global park_hart
park_hart:
    wfi
    j park_hart
