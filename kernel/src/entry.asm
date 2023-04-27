# Disable generation of compressed instructions
.option norvc

.section .text.init
.global _start
_start:
	# Stop the assembler from assuming this is already initialized
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

    # Delegate all traps to Supervisor
    li t0, 0xffffffffffffff
    csrw medeleg, t0
    csrw mideleg, t0

    # Set the Physical Memory Protection to allow Supervisor to access all memory
    csrwi pmpcfg0, 0xf

    # Allow Supervisor to access counter registers (`CYCLE`, `TIME`, `INSTRET`)
    csrwi mcounteren, 1 | 1 << 1 | 1 << 2
    csrwi scounteren, 1 << 1 # Only allow `TIME` for User

    # Set the Machine trap vector
    la t0, machine_trap_vector
    csrw mtvec, t0

    # Initialize timer interrupts
    call machine_timer_init

	# Temporarily disable paging, will be enabled by the kernel once its ready
	csrw satp, zero

	# Set up the jump to the kernels entry point
	la t0, kernel_main
	csrw mepc, t0

	# Place to continue execution after the kernel has finished (should never be reached)
	la ra, park_hart

    # Set the Machine Previous Privilege mode to Supervisor, this will apply once we call `mret`
    li t0, 1 << 11
	csrs mstatus, t0

	# Enter supervisor mode and jump to the kernel
	mret

.section .text.init
park_hart:
    wfi
    j park_hart
