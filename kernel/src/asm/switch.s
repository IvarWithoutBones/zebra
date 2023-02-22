.global supervisor_trap_vector
.align 16
supervisor_trap_vector:
    # Make room for all our registers on the stack
    addi sp, sp, -256

    # Save all our registers except the thread pointer as we might switch HARTs in the middle of a trap.
    # TODO: do we need to save floating-pointer registers?
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
    call supervisor_trap_handler

    # Restore all our registers
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

    # Restore the stack pointer
    addi sp, sp, 256

    # Return to the point of execution prior to the trap
    sret
