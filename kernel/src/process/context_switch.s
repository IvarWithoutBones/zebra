.align 4
.section .trampoline
.global trampoline
trampoline:
    li t0, {TRAPFRAME_ADDR}

    # Load the kernels SATP
    ld t1, 8(t0)

    # Load the kernels trap vector
    ld t2, 16(t0)

    # Switch to the kernels page table, t0 will be invalid afterwards
    csrw satp, t1
    sfence.vma zero, zero

    # Jump to the kernels trap vector
    jr t2

.section .text
.global user_enter
user_enter:
    # a0: trap frame
    # a1: trampoline pointer

    # Point the trap vector to the trampoline page
    csrw stvec, a1

    # Store the kernels SATP
    csrr t0, satp
    sd t0, 8(a0)

    # Store the kernels trap vector
    la t0, supervisor_trap_vector
    sd t0, 16(a0)

    # Set the users program counter
    ld t0, 24(a0)
    csrw sepc, t0

    # Set the stack pointer
    ld t0, 32(a0)
    mv sp, t0

    # Switch to the users page table
    ld t0, 0(a0)
    csrw satp, t0
    sfence.vma zero, zero # Flush the TLB

    # Begin executing in user mode
    sret
