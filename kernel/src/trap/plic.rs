#![allow(dead_code)]

use {
    crate::uart,
    arbitrary_int::{u10, u3},
};

pub const BASE_ADDR: usize = 0x0c00_0000;

pub unsafe fn init() {
    let uart_id = u10::new(uart::IRQ_ID as _);
    set_threshold(u3::new(0));
    set_priority(uart_id, u3::new(1));
    enable(uart_id);
}

#[repr(usize)]
pub enum Registers {
    SupervisorEnable = 0x2080,
    SupervisorPriority = 0x201000,
    SupervisorClaim = 0x201004,
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

/// Claim the next interrupt for context 1
pub fn claim() -> Option<u32> {
    let id = unsafe { Registers::SupervisorClaim.read() };
    if id != 0 {
        Some(id)
    } else {
        None
    }
}

/// Complete an interrupt for context 1. ID should come from `claim()`
pub fn complete(id: u32) {
    unsafe { Registers::SupervisorClaim.write(id) }
}
