set disassembly-flavor intel
set disassembler-options priv-spec=1.12

set print asm-demangle on
set print pretty on
set print array on

set tui active-border-mode normal
set tui compact-source on
set max-value-size unlimited
set pagination off

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

define less
    if $argc != 0
        echo "Usage: less <file>\n"
        return
    else
        || less
        refresh
    end
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
    tui reg all
end
