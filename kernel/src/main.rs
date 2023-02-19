#![feature(panic_info_message, custom_test_frameworks, fn_align)]
#![reexport_test_harness_main = "test_entry_point"]
#![test_runner(language_items::test_runner)]
#![no_std]
#![no_main]

#[macro_use]
mod language_items;
mod memory;
mod plic;
mod power;
mod spinlock;
mod uart;

extern crate alloc;

use core::arch::{asm, global_asm};

global_asm!(include_str!("./asm/entry.s"));
global_asm!(include_str!("./asm/switch.s"));

#[no_mangle]
#[repr(align(16))]
extern "C" fn machine_trap_vector() {
    let cause = unsafe {
        let cause: usize;
        asm!("csrr {}, mcause", out(reg) cause);
        cause
    };

    let value = unsafe {
        let value: usize;
        asm!("csrr {}, mtval", out(reg) value);
        value
    };

    unreachable!("machine trap: {cause} ({value:#x})");
}

#[no_mangle]
extern "C" fn supervisor_trap_handler() {
    let cause = unsafe {
        let cause: usize;
        asm!("csrr {}, scause", out(reg) cause);
        cause
    };

    let value = unsafe {
        let value: usize;
        asm!("csrr {}, stval", out(reg) value);
        value
    };

    // This reads PLIC context 1, which for some reason seems to suffice for the interrupt claiming process?
    // The PLIC will continue to send interrupts to the CPU until the interrupt is acknowledged, which will
    // cause the trap handler to be called in a loop, eventually overflowing the stack.
    let _ = unsafe { ((0x0c00_0000 + 0x201004) as *mut u32).read_volatile() };

    println!("supervisor trap: {cause} ({value:#x})");
}

#[no_mangle]
extern "C" fn kernel_main() {
    uart::UART.lock_with(|uart| uart.init());
    println!("kernel_main() called, we have reached Rust!");

    unsafe {
        // Set the supervisor trap handler
        asm!("la t0, supervisor_trap_vector");
        asm!("csrw stvec, t0");

        memory::init();
        plic::init();
    }

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
