pub mod plic;

use {crate::memory::page, core::arch::asm};

pub unsafe fn init() {
    // Set the Supervisor trap handler defined in `switch.s`, which will execute
    // the `supervisor_trap_handler` function. Note that the Machine trap vector is set inside of `entry.s`.
    asm!("la t0, supervisor_trap_vector");
    asm!("csrw stvec, t0");
}

pub unsafe fn enable_interrupts() {
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

#[derive(Debug, PartialEq, Eq)]
enum Interrupt {
    SupervisorSoftware = 1,
    SupervisorTimer = 5,
    SupervisorExternal = 9,
}

impl From<usize> for Interrupt {
    fn from(cause: usize) -> Self {
        match cause {
            1 => Interrupt::SupervisorSoftware,
            5 => Interrupt::SupervisorTimer,
            9 => Interrupt::SupervisorExternal,
            _ => unreachable!("invalid interrupt value: {}", cause),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Exception {
    InstructionAddressMisaligned,
    InstructionAccessFault,
    IllegalInstruction,
    Breakpoint,
    LoadAddressMisaligned,
    LoadAccessFault,
    StoreAmoAddressMisaligned,
    StoreAmoAccessFault,
    UserEnvironmentCall,
    SupervisorEnvironmentCall,
    InstructionPageFault,
    LoadPageFault,
    StoreAmoPageFault,
}

impl From<usize> for Exception {
    fn from(cause: usize) -> Self {
        match cause {
            0 => Self::InstructionAddressMisaligned,
            1 => Self::InstructionAccessFault,
            2 => Self::IllegalInstruction,
            3 => Self::Breakpoint,
            4 => Self::LoadAddressMisaligned,
            5 => Self::LoadAccessFault,
            6 => Self::StoreAmoAddressMisaligned,
            7 => Self::StoreAmoAccessFault,
            8 => Self::UserEnvironmentCall,
            9 => Self::SupervisorEnvironmentCall,
            12 => Self::InstructionPageFault,
            13 => Self::LoadPageFault,
            15 => Self::StoreAmoPageFault,
            _ => unreachable!("invalid exception value: {}", cause),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Trap {
    Interrupt(Interrupt),
    Exception(Exception),
}

impl Trap {
    const fn is_interrupt(trap_cause: usize) -> bool {
        // The highest bit denotes whether the trap is an interrupt or an exception.
        (trap_cause & (1 << (usize::BITS - 1))) != 0
    }

    const fn code(trap_cause: usize) -> usize {
        trap_cause & ((1 << (usize::BITS - 1)) - 1)
    }
}

impl From<usize> for Trap {
    fn from(trap_cause: usize) -> Self {
        let code = Self::code(trap_cause);
        if Self::is_interrupt(trap_cause) {
            Self::Interrupt(Interrupt::from(code))
        } else {
            Self::Exception(Exception::from(code))
        }
    }
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

    let trap = Trap::from(cause);

    if let Trap::Interrupt(ref intr) = trap {
        match *intr {
            Interrupt::SupervisorExternal => {
                // This reads PLIC context 1, which for some reason seems to suffice for the interrupt claiming process?
                // The PLIC will continue to send interrupts to the CPU until the interrupt is acknowledged, which will
                // cause the trap handler to be called in a loop, eventually overflowing the stack.
                unsafe { ((0x0c00_0000 + 0x201004) as *mut u32).read_volatile() };
            }

            _ => panic!("unhandled interrupt: {:?}", intr),
        }
    } else if let Trap::Exception(ref excp) = trap {
        if let Some(paddr) = unsafe { (*page::root_table()).physical_addr_of(value) } {
            println!("trap {excp:?}: stval={value:#x} paddr={paddr:#x}");
            return;
        };
    }

    println!("trap {trap:?}: stval={value:#x}");
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
