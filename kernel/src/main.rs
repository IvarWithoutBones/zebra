#![no_std]
#![no_main]
#![feature(panic_info_message)]
// Tests dont do anything for now, but this shuts up rust-analyzer
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#[cfg(test)]
fn test_runner(_tests: &[&dyn Fn()]) {}

mod spinlock;
mod uart;

use core::{
    arch::{asm, global_asm},
    panic::PanicInfo,
};

global_asm!(include_str!("./asm/entry.s"));

#[no_mangle]
unsafe extern "C" fn park_hart() -> ! {
    unsafe {
        asm!("wfi", "j {park}", park = sym park_hart, options(noreturn));
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        print!("panicked: {}", s);
    } else {
        if let Some(msg) = info.message() {
            print!("panicked at '");
            if let Some(str) = msg.as_str() {
                print!("{}", str);
            } else {
                print!("unknown");
            }
            print!("', ");
        }

        if let Some(loc) = info.location() {
            print!("{}:{}:{}", loc.file(), loc.line(), loc.column());
        }
    }
    println!();
    unsafe { park_hart() }
}

#[no_mangle]
extern "C" fn kernel_main() {
    uart::UART.lock_with(|uart| uart.init());
    println!("kernel_main() called, we have reached Rust!");

    unsafe {
        asm!("li t4, 0xFEEDFACECAFEBEEF");
    }

    loop {
        if let Some(c) = uart::UART.lock_with(|uart| uart.poll()) {
            println!("got char: '{}' (0x{:02X})", c as char, c);
            if c == b'q' {
                break;
            }
        }
    }
    panic!("intended panic because we shutdown technology is for fools");
}
