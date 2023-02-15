#![feature(panic_info_message, custom_test_frameworks, fn_align)]
#![reexport_test_harness_main = "test_entry_point"]
#![test_runner(language_items::test_runner)]
#![no_std]
#![no_main]

extern crate alloc;

#[macro_use]
mod language_items;
mod memory;
mod power;
mod sections;
mod spinlock;
mod uart;

use core::arch::{asm, global_asm};

global_asm!(include_str!("./asm/entry.s"));

#[no_mangle]
#[repr(align(4))]
extern "C" fn m_trap_vector() {
    let cause = unsafe {
        let cause: usize;
        asm!("csrr {}, mcause", out(reg) cause);
        cause
    };
    unreachable!("machine trap: {cause}");
}

#[repr(align(4))]
extern "C" fn s_trap_vector() {
    let cause = unsafe {
        let cause: usize;
        asm!("csrr {}, scause", out(reg) cause);
        cause
    };
    println!("supervisor trap: {cause}");
}

#[no_mangle]
extern "C" fn kernel_main() {
    uart::UART.lock_with(|uart| uart.init());
    println!("kernel_main() called, we have reached Rust!");

    // Set the supervisor trap handler
    unsafe {
        asm!("la a0, {}", sym s_trap_vector);
        asm!("csrw stvec, a0");
    }

    unsafe { memory::init() };

    // Start executing the reexported test harness's entry point.
    // This will shut down the system when testing is complete.
    #[cfg(test)]
    test_entry_point();

    let mut some_container = alloc::vec::Vec::new();

    loop {
        if let Some(b) = uart::UART.lock_with(|uart| uart.poll()) {
            let c = b as char;
            println!("got char: '{c}' (0x{b:02X})");

            match c {
                'q' => {
                    println!("shutting down");
                    power::shutdown(power::ExitType::Success);
                }

                'r' => {
                    println!("rebooting");
                    power::shutdown(power::ExitType::Reboot);
                }

                'p' => {
                    println!("characters: {some_container:?}");
                }

                'b' => break,

                _ => some_container.push(c),
            }
        }
    }

    panic!("intended panic because we shutdown technology is for fools");
}

#[cfg(test)]
mod tests {
    #[test_case]
    fn basic() {
        assert!(true);
    }
}
