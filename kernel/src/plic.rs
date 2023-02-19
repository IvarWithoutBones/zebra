#![allow(dead_code)]

use {
    arbitrary_int::{u10, u3},
    core::arch::asm,
};

// TODO: move?
const UART_IRQ_ID: usize = 10;
pub const BASE_ADDR: usize = 0x0c00_0000;

#[repr(usize)]
pub enum Registers {
    SupervisorEnable = 0x2080,
    SupervisorPriority = 0x201000,
}

impl Registers {
    unsafe fn as_ptr(self) -> *mut u32 {
        (BASE_ADDR + (self as usize)) as _
    }

    pub unsafe fn read(self) -> u32 {
        self.as_ptr().read_volatile()
    }

    unsafe fn write<T>(self, value: T)
    where
        T: Into<u32>,
    {
        self.as_ptr().write_volatile(value.into())
    }
}

/// Set the source priority for the given interrupt ID
fn set_priority(interrupt_id: u10, priority: u3) {
    unsafe {
        (BASE_ADDR as *mut u32)
            .add(interrupt_id.value().into())
            .write_volatile(priority.into());
    }
}

// TODO: dont hardcode context 1

/// Set the threshold for context 1
fn set_threshold(threshold: u3) {
    unsafe {
        Registers::SupervisorPriority.write(threshold);
    }
}

/// Set the enable bit for the given interrupt ID on context 1
fn enable(interrupt_id: u10) {
    let id: u32 = 1 << interrupt_id.value();
    unsafe {
        Registers::SupervisorEnable.write(id);
    }
}

pub unsafe fn init() {
    let uart_id = u10::new(UART_IRQ_ID as _);
    set_threshold(u3::new(0));
    set_priority(uart_id, u3::new(1));
    enable(uart_id);

    // Enable interrupts
    let sstatus = {
        let sstatus: usize;
        asm!("csrr {}, sstatus", out(reg) sstatus);
        sstatus
    };
    asm!("csrw sstatus, {}", in(reg) sstatus | 1 << 1);
    asm!("csrw sie, {}", in(reg) 1 << 9);
}
