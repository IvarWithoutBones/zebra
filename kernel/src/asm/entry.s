# Disable generation of compressed instructions.
.option norvc

.section .text.init

.global _start
_start:
	# Disable linker instruction relaxation for the `la` instruction below,
	# this disallows the assembler from assuming that `gp` is already initialized.
    .option push
    .option norelax
	la gp, _global_pointer
    .option pop

	# Should be zero, but let's make sure
	csrw satp, zero

	# Any hardware threads (hart) other than hart 0 should park
	csrr t0, mhartid
	bnez t0, park

	# Clear the BSS section
	la a0, _bss_start
	la a1, _bss_end
clear_bss_loop:
	sd zero, (a0)
	addi a0, a0, 8
	bltu a0, a1, clear_bss_loop

    # Initialize the stack
	la sp, _stack_end

	# 0b11 << 11: Machine's previous protection mode is 3 (MPP=3).
	li t0, 0b11 << 11 | (1 << 13)
	csrw mstatus, t0

	# Machine's exception program counter (MEPC) is set to `kernel_main`, our Rust entry point.
    # We will jump to this when we return.
	la t1, kernel_main
	csrw mepc, t1

	# Set up the trap handler, will be used for interrupts.
	la t2, trap_vector
	csrw mtvec, t2

	# Place to continue execution after `mepc` returns.
	la ra, park

	# Update mstatus and jump to `mepc`
	mret

park:
	wfi
	j park

# This will be called upon interrupts, no-op for now
.global trap_vector
trap_vector:
    mret
