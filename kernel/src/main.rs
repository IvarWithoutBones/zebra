#![feature(panic_info_message, custom_test_frameworks)]
#![reexport_test_harness_main = "test_entry_point"]
#![test_runner(language_items::test_runner)]
#![no_std]
#![no_main]

#[macro_use]
mod language_items;
mod page;
mod power;
mod spinlock;
mod uart;

use core::arch::{asm, global_asm};

global_asm!(include_str!("./asm/entry.s"));
global_asm!(include_str!("./asm/symbols.s"));

#[no_mangle]
extern "C" fn kernel_main() {
    uart::UART.lock_with(|uart| uart.init());
    println!("kernel_main() called, we have reached Rust!");

    // Start executing the reexported test harness's entry point.
    // This will shut down the system when testing is complete.
    #[cfg(test)]
    test_entry_point();

    unsafe {
        asm!("li t4, 0xFEEDFACECAFEBEEF");
    }

    let mut pages: [Option<*mut u8>; 1024] = [None; 1024];
    let mut page_idx = 0;
    loop {
        if let Some(c) = uart::UART.lock_with(|uart| uart.poll()) {
            println!("got char: '{}' (0x{:02X})", c as char, c);
            if c == b'q' {
                println!("shutting down");
                power::shutdown(power::ExitType::Success);
            } else if c == b'r' {
                println!("rebooting");
                power::shutdown(power::ExitType::Reboot);
            } else if c == b'a' {
                let ptr = page::allocate(2).unwrap();
                println!();
                page::print();
                pages[page_idx] = Some(ptr);
                page_idx += 1;
                println!()
            } else if c == b'f' {
                assert!(page_idx > 0);
                page_idx -= 1;
                let ptr = pages[page_idx].unwrap();
                page::deallocate(ptr);
                println!();
                page::print();
                println!()
            } else if c == b'p' {
                break;
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
