use super::{switch_into, user_trap_vector, TRAPFRAME_PTR};
use crate::memory::page;
use alloc::boxed::Box;
use bitbybit::bitenum;
use core::{
    arch::global_asm,
    fmt,
    ops::{Index, IndexMut},
};

global_asm!(include_str!("switch.asm"), TRAPFRAME_PTR = const TRAPFRAME_PTR);

/// NOTE: the numbering of these registers is important, as they are used as offsets in assembly.
#[bitenum(u64)]
#[derive(Debug)]
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
        31
    }
}

#[derive(Default, PartialEq, Eq)]
#[repr(C)]
pub struct UserState {
    registers: [u64; Registers::len()],
}

impl UserState {
    pub fn new(stack_start: *const u8) -> Self {
        let mut result = Self::default();
        result[Registers::StackPointer] = stack_start as _;
        result
    }
}

impl UserState {
    pub fn iter_names(&self) -> impl Iterator<Item = (Registers, &u64)> {
        self.registers
            .iter()
            .enumerate()
            .map(|(i, data)| (Registers::new_with_raw_value(i as _).unwrap(), data))
    }
}

impl Index<Registers> for UserState {
    type Output = u64;

    fn index(&self, index: Registers) -> &Self::Output {
        &self.registers[index as usize]
    }
}

impl IndexMut<Registers> for UserState {
    fn index_mut(&mut self, index: Registers) -> &mut Self::Output {
        &mut self.registers[index as usize]
    }
}

#[derive(PartialEq, Eq)]
#[repr(C)]
struct KernelState {
    satp: u64,
    trap_vector_ptr: u64,
    stack_start: *const u8,
}

impl KernelState {
    fn new(stack_start: *const u8) -> Self {
        Self {
            satp: page::root_table().build_satp() as _,
            trap_vector_ptr: user_trap_vector as *const u8 as _,
            stack_start,
        }
    }
}

#[repr(C)]
#[derive(Debug, PartialEq, Eq)]
pub struct TrapFrame {
    kernel_state: KernelState,
    pub user_state: UserState,
}

impl TrapFrame {
    pub fn new(kernel_stack: *const u8, user_stack: *const u8) -> Box<Self> {
        Box::new(Self {
            kernel_state: KernelState::new(kernel_stack),
            user_state: UserState::new(user_stack),
        })
    }

    pub fn set_user_satp(&mut self, satp: u64) {
        self.user_state[Registers::Satp] = satp;
    }

    pub fn as_ptr(&self) -> *const Self {
        self as *const _
    }

    pub unsafe fn run(&self) -> ! {
        switch_into(self)
    }
}

impl fmt::Debug for UserState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "UserState {{ ")?;
        for (name, value) in self.iter_names() {
            writeln!(f, "    {name:?}: {value:#x}, ")?;
        }
        write!(f, "}}")?;
        Ok(())
    }
}

impl fmt::Debug for KernelState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("KernelState")
            .field("satp", &format_args!("{:#x}", self.satp))
            .field(
                "trap_vector_ptr",
                &format_args!("{:#x}", self.trap_vector_ptr),
            )
            .field("stack_start", &format_args!("{:#p}", self.stack_start))
            .finish()
    }
}
