OUTPUT_ARCH("riscv")
ENTRY(_start)

MEMORY {
    /* Use the memory mapping qemu expects */
    ram (wxa) : ORIGIN = 0x80000000, LENGTH = 128M
}

/* Sections exposed through the program headers */
PHDRS {
    text PT_LOAD;
    rodata PT_LOAD;
    data PT_LOAD;
    bss PT_LOAD;
}

SECTIONS {
    .text : {
        PROVIDE(_text_start = .);

        *(.text.init) /* Entry point, must match up with `ram.ORIGIN` */
        *(.text .text.*)

        /* Trampoline code, must be page aligned */
        . = ALIGN(0x1000);
        PROVIDE(_trampoline_start = .);
        *(.trampoline)
        . = ALIGN(0x1000);
        PROVIDE(_trampoline_end = .);
        ASSERT(_trampoline_end - _trampoline_start == 0x1000, "trampoline is not page sized")

        PROVIDE(_text_end = .);
    } >ram AT>ram :text

    .rodata : {
        PROVIDE(_rodata_start = .);

        *(.rodata .rodata.*)

        PROVIDE(_rodata_end = .);
    } >ram AT>ram :rodata

    .data : {
        . = ALIGN(0x1000);
        PROVIDE(_data_start = .);

        *(.sdata .sdata.*)
        *(.data .data.*)

        PROVIDE(_data_end = .);
    } >ram AT>ram :data

    .bss : {
        PROVIDE(_bss_start = .);

        *(.sbss .sbss.*)
        *(.bss .bss.*)

        PROVIDE(_bss_end = .);
    } >ram AT>ram :bss

    /* Some helpful symbols the kernel can use for memory mapping */

    PROVIDE(_global_pointer = _text_end); /* Base for relative offsets */

    PROVIDE(_memory_start = ORIGIN(ram));
    PROVIDE(_memory_end = ORIGIN(ram) + LENGTH(ram));

    PROVIDE(_stack_start = _bss_end + 0x1000 /* Leave room for a guard page */);
    PROVIDE(_stack_end = _stack_start + (64 * 0x1000) /* 256 KiB */);

    PROVIDE(_heap_start = _stack_end);
    PROVIDE(_heap_end = _memory_end);
}
