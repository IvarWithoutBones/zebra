#![allow(dead_code)]

use {
    crate::memory::{allocator, page},
    alloc::boxed::Box,
};

const STACK_SIZE: usize = 4096;
const STACK_ADDR: usize = 0x2000_0000;
const PROGRAM_ADDR: usize = 0x1000_0000;

#[derive(Debug)]
#[repr(C)]
pub struct TrapFrame {
    pub registers: [usize; 32],       // 0 - 255
    pub float_registers: [usize; 32], // 256 - 511
    pub satp: usize,                  // 512 - 519
    pub trap_stack: *mut u8,          // 520
    pub hartid: usize,                // 528
}

impl TrapFrame {
    const fn new() -> Self {
        Self {
            registers: [0; 32],
            float_registers: [0; 32],
            satp: 0,
            trap_stack: core::ptr::null_mut(),
            hartid: 0,
        }
    }
}

#[derive(Debug)]
enum ProcessState {
    Running,
    Sleeping,
    Waiting,
    Dead,
}

#[derive(Debug)]
#[repr(C)]
pub struct Process {
    state: ProcessState,
    pid: usize,
    stack: *mut u8,
    program_counter: usize,
    page_table: Box<page::Table>,
    trap_frame: Box<TrapFrame>,
}

impl Process {
    pub fn new(func: fn()) -> Self {
        let mut proc = Self {
            state: ProcessState::Waiting,
            pid: 0,
            stack: allocator().allocate(STACK_SIZE).unwrap(),
            program_counter: func as usize,
            page_table: Box::new(page::Table::new()),
            trap_frame: Box::new(TrapFrame::new()),
        };

        // Set the stack pointer (x2)
        proc.trap_frame.registers[2] = STACK_ADDR + STACK_SIZE;

        // Map the stack
        for i in 0..STACK_SIZE {
            proc.page_table.kernel_map(
                STACK_ADDR + i,
                proc.stack as usize + i,
                page::EntryAttributes::UserReadWrite as _,
            );
        }

        // Map the program
        proc.page_table.kernel_map(
            PROGRAM_ADDR,
            func as usize,
            page::EntryAttributes::UserReadExecute as _,
        );

        proc
    }
}
