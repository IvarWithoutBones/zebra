OUTPUT_ARCH("riscv")
ENTRY(_start)

MEMORY {
    ram (wxa) : ORIGIN = 0x200000, LENGTH = 16M
}

PHDRS {
    text PT_LOAD;
    rodata PT_LOAD;
    data PT_LOAD;
    bss PT_LOAD;
}

SECTIONS {
    .text : {
        . = ALIGN(0x1000);
        *(.text .text.*)
        . = ALIGN(0x1000);
    } >ram AT>ram :text

    .rodata : {
        . = ALIGN(0x1000);
        *(.rodata .rodata.*)
        . = ALIGN(0x1000);
    } >ram AT>ram :rodata

    .data : {
        . = ALIGN(0x1000);
        *(.sdata .sdata.*)
        . = ALIGN(0x1000);
        *(.data .data.*)
        . = ALIGN(0x1000);
    } >ram AT>ram :data

    .bss : {
        . = ALIGN(0x1000);
        *(.sbss .sbss.*)
        . = ALIGN(0x1000);
        *(.bss .bss.*)
        . = ALIGN(0x1000);
    } >ram AT>ram :bss
}
