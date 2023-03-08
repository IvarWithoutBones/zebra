set disassembly-flavor intel
set disassembler-options priv-spec=1.12

set print asm-demangle on
set print pretty on
set print array on
set print symbol on

set tui active-border-mode normal
set tui compact-source on
set max-value-size unlimited
set pagination off
set confirm off
set disassemble-next-line on
set step-mode on

define regs
    info registers
    info registers mscratch
    info registers sscratch
    info registers mtvec
    info registers stvec
    info registers mepc
    info registers sepc
    info registers mtval
    info registers stval
    info registers mcause
    info registers scause
    info registers satp
    info registers mhartid
    info registers mstatus
    info registers priv
end

document regs
    Print relevant RISC-V registers and CSR's
end

define qa
    kill
    exit
end

document qa
    Kill the attached process and exit gdb
end

# Comparing definition arguments as strings unfortunately doesnt work on platforms without `malloc`.
# We have to work around this by defining multiple functions, really just as verbose as it gets :/

define physmem
    if $argc != 0
        help physmem
        return
    end

    echo 0 = virtual, 1 = physical\n\n
    maintenance packet qqemu.PhyMemMode
end

document physmem
    Print the memory protection mode, where 1 is physical and 0 is virtual.
end

define physmem_on
    if $argc != 0
        help physmem_on
        return
    end

    maintenance packet Qqemu.PhyMemMode:1
end

document physmem_on
    Turn on memory protection.
end

define physmem_off
    if $argc != 0
        help physmem_off
        return
    end

    maintenance packet Qqemu.PhyMemMode:0
end

document physmem_on
    Turn off memory protection.
end

define sstep
    if $argc != 0
        help sstep
        return
    end

    maintenance packet qqemu.sstepbits
    echo \n
    maintenance packet qqemu.sstep
end

document sstep
    Print the single-step behaviour bits.
end

define sstep_set
    if $argc != 1
        help sstep_set
        return
    end

    maintenance packet Qqemu.sstep=$arg0
end

document sstep_set
    Set the single-step behaviour bits.
end

tui new-layout full {-horizontal asm 1 src 1} 2 status 0 cmd 1
tui new-layout asm {-horizontal asm 1 regs 1} 2 status 0 cmd 1

define lfull
    layout full
end

define lempty
    tui disable
end

define lsrc
    layout src
end

define lasm
    layout asm
end
