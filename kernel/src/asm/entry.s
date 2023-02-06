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

	# Should be zero but let's make sure
	csrw satp, zero

	# Any hardware threads (hart) other than hart 0 should park
	csrr t0, mhartid
	bnez t0, park_hart

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

	# Machine's exception program counter (MEPC) is set to our Rust entry point,
    # we will jump to this when we call `mret` below.
	la t1, kernel_main
	csrw mepc, t1

	# Set up the trap handler, will be used for interrupts.
	la t2, trap_vector
	csrw mtvec, t2

	# Place to continue execution after `mepc` returns, this should never be reached.
	la ra, park_hart

	# Update mstatus and jump to `mepc`
	mret

# This will be called upon interrupts, no-op for now
.global trap_vector
trap_vector:
    call print_str
    mret

# Random stuff to get familiar with ASM

.align 3
test: .ascii "this is being printed from ASM\n\0"

print_str:
    la t0, test
    li t1, 0x10000000 # UART

    lbu t2, (t0)
print_str_loop:
    sb t2, (t1) # Write current character to UART
    addi t0, t0, 1 # Increment pointer to next character
    lbu t2, (t0) # Load next character
    bnez t2, print_str_loop # Repeat unless we hit a null

    mret
