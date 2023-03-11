.align 4
.section .trampoline
.global user_trap_vector
user_trap_vector:
    # Save the users a0 since we'll need to use it
    csrw sscratch, a0

    li a0, {TRAPFRAME_ADDR}

    # Save the users registers
    sd sp, 40(a0)
    sd ra, 48(a0)
    sd gp, 56(a0)
    sd tp, 64(a0)
    # a0 is skipped for now
    sd a1, 80(a0)
    sd a2, 88(a0)
    sd a3, 96(a0)
    sd a4, 104(a0)
    sd a5, 112(a0)
    sd a6, 120(a0)
    sd a7, 128(a0)
    sd t0, 136(a0)
    sd t1, 144(a0)
    sd t2, 152(a0)
    sd t3, 160(a0)
    sd t4, 168(a0)
    sd t5, 176(a0)
    sd t6, 184(a0)
    sd s0, 192(a0)
    sd s1, 200(a0)
    sd s2, 208(a0)
    sd s3, 216(a0)
    sd s4, 224(a0)
    sd s5, 232(a0)
    sd s6, 240(a0)
    sd s7, 248(a0)
    sd s8, 256(a0)
    sd s9, 264(a0)

    # Save the users program counter
    csrr t0, sepc
    sd t0, 32(a0)

    # Load the kernels registers
    ld t1, 8(a0) # Kernel trap handler
    ld sp, 16(a0) # Kernel stack pointer

    # Save the users a0
    csrr t0, sscratch
    sd t0, 72(a0)

    # Save the users SATP
    csrr t0, satp
    sd t0, 24(a0)

    # Switch to the kernels page table, a0 will be invalid afterwards
    ld t0, 0(a0) # Kernel SATP
    sfence.vma zero, zero
    csrw satp, t0

    # Call `user_trap_handler`
    csrr a0, scause
    jr t1

.section .text
.global user_enter
user_enter:
    # a0: pointer to a TrapFrame

    li ra, 0 # avoid confusion

    # Store the kernels SATP
    csrr t0, satp
    sd t0, 0(a0)

    # Store the kernels trap vector
    la t0, user_trap_handler
    sd t0, 8(a0)

    # TODO: doing this breaks things, presumably overflow?
    # Store the kernels stack pointer
    # sd sp, 16(a0)

    # Switch to the users page table
    ld a0, 24(a0)
    sfence.vma zero, zero # Flush the TLB
    csrw satp, a0

    # Redirect traps to the trampoline
    la t0, user_trap_vector
    csrw stvec, t0

    li a0, {TRAPFRAME_ADDR}

    # Load the users program counter
    ld t0, 32(a0)
    csrw sepc, t0

    # Load the users registers
    ld sp, 40(a0)
    ld ra, 48(a0)
    ld gp, 56(a0)
    ld tp, 64(a0)
    # a0 is skipped for now
    ld a1, 80(a0)
    ld a2, 88(a0)
    ld a3, 96(a0)
    ld a4, 104(a0)
    ld a5, 112(a0)
    ld a6, 120(a0)
    ld a7, 128(a0)
    ld t0, 136(a0)
    ld t1, 144(a0)
    ld t2, 152(a0)
    ld t3, 160(a0)
    ld t4, 168(a0)
    ld t5, 176(a0)
    ld t6, 184(a0)
    ld s0, 192(a0)
    ld s1, 200(a0)
    ld s2, 208(a0)
    ld s3, 216(a0)
    ld s4, 224(a0)
    ld s5, 232(a0)
    ld s6, 240(a0)
    ld s7, 248(a0)
    ld s8, 256(a0)
    ld s9, 264(a0)

    ld a0, 72(a0) # Finally load the users a0

    # Set the Previous Privilege Mode to User
    csrc sstatus, 8

    # Begin executing in user mode
    sret
