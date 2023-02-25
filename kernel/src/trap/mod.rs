mod clint;
pub mod plic;

use {
    crate::uart,
    core::{arch::asm, fmt::Debug},
};

extern "C" {
    // Trap handler defined in `switch.s`
    fn supervisor_trap_vector();
}

pub unsafe fn init() {
    // Set the supervisor trap handler defined in `switch.s`, which will execute the handler below
    asm!("csrw stvec, {}", in(reg) supervisor_trap_vector as usize);
}

pub unsafe fn enable_interrupts() {
    println!("initializing PLIC...");
    plic::init();
    println!("PLIC initialized");

    println!("enabling interrupts...");

    // Set the interrupt enable bit
    asm!("csrs sstatus, {}", in(reg) 1 << 1);
    // Enable external, timer, and software interrupts
    asm!("csrs sie, {}", in(reg) 1 << 9 | 1 << 5 | 1 << 1);

    println!("interrupts enabled");
}

#[derive(Debug, PartialEq, Eq)]
enum Interrupt {
    SupervisorSoftware,
    SupervisorTimer,
    SupervisorExternal,
}

impl From<usize> for Interrupt {
    fn from(code: usize) -> Self {
        match code {
            1 => Interrupt::SupervisorSoftware,
            5 => Interrupt::SupervisorTimer,
            9 => Interrupt::SupervisorExternal,
            _ => unreachable!("invalid interrupt code: {code}"),
        }
    }
}

impl Interrupt {
    fn handle(&self) {
        match self {
            Self::SupervisorExternal => {
                if let Some(intr) = plic::claim() {
                    match intr as usize {
                        uart::IRQ_ID => uart::interrupt(),
                        _ => panic!("unhandled external interrupt: {intr}"),
                    }

                    plic::complete(intr);
                }
            }

            Self::SupervisorSoftware => {
                // Clear the interrupt pending bit
                unsafe { asm!("csrc sip, 2") }
                println!("timer interrupt");
            }

            _ => panic!("unhandled interrupt: {self:?}"),
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
    fn from(code: usize) -> Self {
        match code {
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
            _ => unreachable!("invalid exception code: {code}"),
        }
    }
}

impl Exception {
    fn handle(&self) {
        let stval = unsafe {
            let value: usize;
            asm!("csrr {}, stval", lateout(reg) value);
            value
        };

        panic!("unhandled exception: {self:?} (stval={stval:#x})");
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Trap {
    Interrupt(Interrupt),
    Exception(Exception),
}

impl Trap {
    fn handle(&self) {
        match self {
            Self::Interrupt(intr) => intr.handle(),
            Self::Exception(excp) => excp.handle(),
        }
    }

    const fn is_interrupt(cause: usize) -> bool {
        // The highest bit denotes whether the trap is an interrupt or an exception.
        (cause & (1 << (usize::BITS - 1))) != 0
    }

    const fn code(cause: usize) -> usize {
        // Mask off the identifier bit
        cause & ((1 << (usize::BITS - 1)) - 1)
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

    Trap::from(cause).handle();
}
