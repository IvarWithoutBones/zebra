.align 4
.section .text.vectors
.global supervisor_trap_vector
supervisor_trap_vector:
    # Make room for all our registers on the stack
    addi sp, sp, -256

    # Save all our registers except the thread pointer as we might switch HARTs in the middle of a trap.
    sd ra, 0(sp)
    sd sp, 8(sp)
    sd gp, 16(sp)
    sd t0, 32(sp)
    sd t1, 40(sp)
    sd t2, 48(sp)
    sd t3, 56(sp)
    sd t4, 64(sp)
    sd t5, 72(sp)
    sd t6, 80(sp)
    sd s0, 88(sp)
    sd s1, 96(sp)
    sd s2, 104(sp)
    sd s3, 112(sp)
    sd s4, 120(sp)
    sd s5, 128(sp)
    sd s6, 136(sp)
    sd s7, 144(sp)
    sd s8, 152(sp)
    sd s9, 160(sp)
    sd s10, 168(sp)
    sd s11, 176(sp)
    sd a0, 184(sp)
    sd a1, 192(sp)
    sd a2, 200(sp)
    sd a3, 216(sp)
    sd a4, 224(sp)
    sd a5, 232(sp)
    sd a6, 240(sp)
    sd a7, 248(sp)

    # Call the kernels trap handler
    csrr a0, scause
    call supervisor_trap_handler

    # Restore all our registers
    ld ra, 0(sp)
    ld sp, 8(sp)
    ld gp, 16(sp)
    ld t0, 32(sp)
    ld t1, 40(sp)
    ld t2, 48(sp)
    ld t3, 56(sp)
    ld t4, 64(sp)
    ld t5, 72(sp)
    ld t6, 80(sp)
    ld s0, 88(sp)
    ld s1, 96(sp)
    ld s2, 104(sp)
    ld s3, 112(sp)
    ld s4, 120(sp)
    ld s5, 128(sp)
    ld s6, 136(sp)
    ld s7, 144(sp)
    ld s8, 152(sp)
    ld s9, 160(sp)
    ld s10, 168(sp)
    ld s11, 176(sp)
    ld a0, 184(sp)
    ld a1, 192(sp)
    ld a2, 200(sp)
    ld a3, 216(sp)
    ld a4, 224(sp)
    ld a5, 232(sp)
    ld a6, 240(sp)
    ld a7, 248(sp)

    # Restore the stack pointer
    addi sp, sp, 256

    # Return to the point of execution prior to the trap
    sret

.align 4
.section .text.vectors
.global machine_trap_vector
machine_trap_vector:
    # Save the registers we will use
    addi sp, sp, -32
    sd a0, 0(sp)
    sd a1, 8(sp)
    sd a2, 16(sp)
    sd a3, 24(sp)

    # Load the context from `clint.rs`, matching the layout defined there:
    #   0: `mtime` pointer
    #   1: `mtimecmp` pointer
    #   2: `mtimecmp` interval
    csrrw a0, mscratch, a0

    # Schedule the next interrupt
    ld a1, 8(a0) # Pointer to mtimecmp
    ld a3, 0(a1) # Dereference the mtimecmp pointer
    ld a2, 16(a0) # Load the interval

    # Add the new interval to mtimecmp
    add a3, a3, a2
    sd a3, 0(a1)

    # Save the context
    csrrw a0, mscratch, a0

    # Raise a supervisor software interrupt
    csrsi sip, 2

    # Restore the used registers
    ld a0, 0(sp)
    ld a1, 8(sp)
    ld a2, 16(sp)
    ld a3, 24(sp)
    addi sp, sp, 32

    # Hand control back to the kernel
    mret
