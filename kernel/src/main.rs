#![no_std]
#![no_main]
#![feature(panic_info_message)]
// Tests dont do anything for now, but this shuts up rust-analyzer
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#[cfg(test)]
fn test_runner(_tests: &[&dyn Fn()]) {}

use core::{
    arch::{asm, global_asm},
    panic::PanicInfo,
};

global_asm!(include_str!("./asm/entry.s"));

const UART: *mut u8 = 0x1000_0000 as _;

fn print(s: &str) {
    unsafe {
        for b in s.bytes() {
            UART.write_volatile(b);
        }
    }
}

fn println(s: &str) {
    print(s);
    print("\n");
}

#[no_mangle]
extern "C" fn print_num(num: usize) {
    const RADIX: usize = 10;
    let tens = num / RADIX;
    if tens > 0 {
        print_num(tens);
    }

    let c = (num % RADIX) as u8 + if num % RADIX < 10 { b'0' } else { b'a' - 10 };
    unsafe {
        UART.write_volatile(c);
    }
}

#[no_mangle]
unsafe extern "C" fn park_hart() -> ! {
    unsafe {
        asm!("wfi", "j {park}", park = sym park_hart, options(noreturn));
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    if let Some(s) = info.payload().downcast_ref::<&str>() {
        print("panicked: ");
        print(s);
    } else {
        if let Some(msg) = info.message() {
            print("panicked at '");
            if let Some(str) = msg.as_str() {
                print(str);
            } else {
                print("unknown");
            }
            print("', ");
        }

        if let Some(loc) = info.location() {
            print(loc.file());
            print(":");
            print_num(loc.line() as _);
            print(":");
            print_num(loc.column() as _);
        }
    }
    print("\n");
    unsafe { park_hart() }
}

#[no_mangle]
extern "C" fn kernel_main() {
    println("kernel_main() called, we have reached Rust!");
    unsafe {
        asm!("li t4, 0xFEEDFACECAFEBEEF");
    }
    panic!("this should panic");
}
