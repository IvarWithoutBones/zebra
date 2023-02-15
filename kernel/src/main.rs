#![feature(panic_info_message, custom_test_frameworks)]
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

use core::arch::global_asm;

global_asm!(include_str!("./asm/entry.s"));

/// Temporarily used for the trap vectors from ASM.
#[no_mangle]
extern "C" fn print_num(num: usize) {
    println!("print_num: {:#X}", num);
}

#[no_mangle]
extern "C" fn kernel_main() {
    uart::UART.lock_with(|uart| uart.init());
    println!("kernel_main() called, we have reached Rust!");

    unsafe { memory::init() };

    // Start executing the reexported test harness's entry point.
    // This will shut down the system when testing is complete.
    #[cfg(test)]
    test_entry_point();

    let mut some_container = alloc::vec::Vec::new();

    loop {
        if let Some(b) = uart::UART.lock_with(|uart| uart.poll()) {
            let c = b as char;
            println!("got char: '{}' (0x{:02X})", c, b);

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
                    println!("characters: {:?}", some_container);
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
