pub mod clint;
pub mod plic;

use crate::memory::page;
use core::{
    arch::{asm, global_asm},
    fmt::Debug,
};

extern "C" {
    // Defined in `vector.s`
    fn supervisor_trap_vector();
}

global_asm!(include_str!("./vector.asm"));

pub unsafe fn attach_supervisor_trap_vector() {
    // Set the supervisor trap vector defined in `vector.s`, which will execute the Rust handler below
    asm!("csrw stvec, {}", in(reg) supervisor_trap_vector as usize);
}

#[cfg(not(test))]
pub fn enable_interrupts() {
    unsafe {
        // Set the interrupt enable bit
        asm!("csrs sstatus, {}", in(reg) 1 << 1);
        // Enable external, timer, and software interrupts
        asm!("csrs sie, {}", in(reg) 1 << 9 | 1 << 5 | 1 << 1);
    }
}

#[allow(clippy::enum_variant_names)] // Just matching the spec
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
            Self::SupervisorExternal => plic::handle_interrupt(),

            Self::SupervisorSoftware => {
                // Clear the interrupt pending bit
                unsafe { asm!("csrc sip, 2") }
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

        let sstatus = unsafe {
            let value: usize;
            asm!("csrr {}, sstatus", lateout(reg) value);
            value
        };

        let sepc = unsafe {
            let value: usize;
            asm!("csrr {}, sepc", lateout(reg) value);
            value
        };

        if let Some(paddr) = page::root_table().physical_addr(sepc) {
            panic!(
                    "unhandled exception: {self:?}, stval={stval:#x}, physical address={paddr:#x}, sstatus={sstatus:#x}",
                );
        } else {
            panic!("unhandled exception: {self:?}, stval={stval:#x}, sstatus={sstatus:#x}");
        }
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

#[no_mangle]
extern "C" fn user_trap_handler(cause: usize) {
    let trap = Trap::from(cause);
    if let Trap::Interrupt(intr) = trap {
        intr.handle();
    }

    crate::thread::handle_trap(cause as _);
}

/// The trap handler for Supervisor mode. This will be called by the respective
/// trap vector after the previous execution context has been saved, and after we
/// return from this function we will restore and resume the previous execution context.
#[no_mangle]
extern "C" fn supervisor_trap_handler(cause: usize) {
    unsafe {
        // Globally disable interrupts, so that a lock being dropped doesnt renable them
        asm!("csrw sie, zero");
    };

    Trap::from(cause).handle();

    unsafe {
        // Enable external, timer, and software interrupts
        asm!("csrs sie, {}", in(reg) 1 << 9 | 1 << 5 | 1 << 1);
    }
}

#[no_mangle]
extern "C" fn machine_trap_handler() {
    let mstatus = unsafe {
        let value: usize;
        asm!("csrr {}, mstatus", lateout(reg) value);
        value
    };

    let cause = unsafe {
        let cause: usize;
        asm!("csrr {}, mcause", lateout(reg) cause);
        cause
    };

    panic!("unhandled machine trap: {cause:#x}, mstatus={mstatus:#x}");
}
