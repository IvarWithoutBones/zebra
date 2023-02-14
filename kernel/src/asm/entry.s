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

	# Any hardware threads (harts) other than hart 0 should park
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

	# Disable paging
	csrw satp, zero

	# Set the machine mode trap handler
	la t0, m_trap_vector
	csrw mtvec, t0

    # Set the Machine Previous Privilege mode to 1 (Supervisor mode), this will apply once we call `mret`.
    li t0, 1 << 11
	csrw mstatus, t0

    # Set the Physical Memory Protection to allow Supervisor mode to access all memory
    li t0, 0x3fffffffffffff
    csrw pmpaddr0, t0
    li t0, 0xf
    csrw pmpcfg0, t0

	# Set the kernels entry point
	la t0, kernel_main
	csrw mepc, t0

	# Place to continue execution after the kernel has finished
	la ra, park_hart

	# Jump to the kernel and enter supervisor mode
	mret

park_hart:
    wfi
    j park_hart

# This will be called upon interrupts/exceptions
.global trap_vector
m_trap_vector:
    csrr a0, mcause
    call print_num
    mret
