pub mod plic;

use {crate::memory::page, core::arch::asm};

pub unsafe fn init() {
    // Set the Supervisor trap handler defined in `switch.s`, which will execute
    // the `supervisor_trap_handler` function. Note that the Machine trap vector is set inside of `entry.s`.
    asm!("la t0, supervisor_trap_vector");
    asm!("csrw stvec, t0");
}

pub unsafe fn init_interrupts() {
    println!("initializing PLIC...");
    plic::init();
    println!("PLIC initialized");

    println!("enabling interrupts...");
    let sstatus = {
        let sstatus: usize;
        asm!("csrr {}, sstatus", out(reg) sstatus);
        sstatus
    };
    asm!("csrw sstatus, {}", in(reg) sstatus | 1 << 1);
    asm!("csrw sie, {}", in(reg) 1 << 9);
    println!("interrupts enabled");
}

/// The trap handler for Supervisor mode. This will be called by the respective
/// trap vector after the previous execution context has been saved, and after we
/// return from this function we will restore and resume the previous execution context.
#[no_mangle]
extern "C" fn supervisor_trap_handler() {
    let cause = unsafe {
        let cause: usize;
        asm!("csrr {}, scause", lateout(reg) cause);
        cause
    };

    let value = unsafe {
        let value: usize;
        asm!("csrr {}, stval", lateout(reg) value);
        value
    };

    // This reads PLIC context 1, which for some reason seems to suffice for the interrupt claiming process?
    // The PLIC will continue to send interrupts to the CPU until the interrupt is acknowledged, which will
    // cause the trap handler to be called in a loop, eventually overflowing the stack.
    let _ = unsafe { ((0x0c00_0000 + 0x201004) as *mut u32).read_volatile() };

    if value != 0 {
        if let Some(paddr) = unsafe { (*page::root_table()).physical_addr_of(value) } {
            println!("supervisor trap: scause={cause} stval={value:#x} paddr={paddr:#x}");
            return;
        };
    }

    println!("supervisor trap: scause={cause} stval={value:#x}");
}

/// The trap vector for Machine mode. This should never be called as
/// all traps are delegated to Supervisor mode, the exception being
/// faults during the boot process (inside `entry.s`).
#[no_mangle]
#[repr(align(16))]
extern "C" fn machine_trap_vector() {
    let cause = unsafe {
        let cause: usize;
        asm!("csrr {}, mcause", lateout(reg) cause);
        cause
    };

    let value = unsafe {
        let value: usize;
        asm!("csrr {}, mtval", lateout(reg) value);
        value
    };

    unreachable!("machine trap: {cause} ({value:#x})");
}
