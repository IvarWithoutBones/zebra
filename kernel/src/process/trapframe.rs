use {super::PROGRAM_START, alloc::boxed::Box, core::fmt};

/// NOTE: Numbering *must* match with the serialisation/deserialisation in `context_switch.s`!
#[repr(u64)]
#[allow(dead_code)]
pub enum Registers {
    Satp = 0,
    ProgramCounter = 1,
    StackPointer = 2,
    ReturnAddress = 3,
    GlobalPointer = 4,
    ThreadPointer = 5,

    A0 = 6,
    A1 = 7,
    A2 = 8,
    A3 = 9,
    A4 = 10,
    A5 = 11,
    A6 = 12,
    A7 = 13,

    T0 = 14,
    T1 = 15,
    T2 = 16,
    T3 = 17,
    T4 = 18,
    T5 = 19,
    T6 = 20,

    S0 = 21,
    S1 = 22,
    S2 = 23,
    S3 = 24,
    S4 = 25,
    S5 = 26,
    S6 = 27,
    S7 = 28,
    S8 = 29,
    S9 = 30,
}

impl Registers {
    pub const fn len() -> usize {
        30
    }
}

#[repr(C)]
#[derive(Default)]
pub struct TrapFrame {
    // Kernel state
    kernel_satp: u64,
    kernel_trap_vector: u64,
    kernel_stack_pointer: u64,
    // User state
    pub registers: [u64; Registers::len()],
}

impl TrapFrame {
    pub fn new(user_satp: u64, stack_pointer: u64, kernel_stack_pointer: u64) -> Box<Self> {
        let mut registers = [0; Registers::len()];
        registers[Registers::Satp as usize] = user_satp;
        registers[Registers::StackPointer as usize] = stack_pointer;
        registers[Registers::ProgramCounter as usize] = PROGRAM_START as _;

        Box::new(Self {
            kernel_stack_pointer,
            registers,
            ..Default::default()
        })
    }

    pub const fn as_ptr(&self) -> *const Self {
        self as *const _
    }
}

impl fmt::Debug for TrapFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrapFrame")
            .field("pointer", &self.as_ptr())
            .field(
                "user_satp",
                &format_args!("{:#X}", self.registers[Registers::Satp as usize]),
            )
            .field(
                "program_counter",
                &format_args!("{:#X}", self.registers[Registers::ProgramCounter as usize]),
            )
            .finish_non_exhaustive()
    }
}
