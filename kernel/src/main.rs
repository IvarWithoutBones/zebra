#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![reexport_test_harness_main = "test_main"]

#[macro_use]
mod uart;
mod power;
mod spinlock;

use core::{
    arch::{asm, global_asm},
    panic::PanicInfo,
};

global_asm!(include_str!("./asm/entry.s"));

#[cfg(test)]
trait Testable {
    fn run(&self) -> ();
}

#[cfg(test)]
impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("{}...\t", core::any::type_name::<T>());
        self();
        println!("[ok]");
    }
}

#[cfg(test)]
fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test.run()
    }
    power::shutdown(power::ExitType::Success)
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{:#}", info);
    power::shutdown(power::ExitType::Failure)
}

#[no_mangle]
extern "C" fn kernel_main() {
    uart::UART.lock_with(|uart| uart.init());
    println!("kernel_main() called, we have reached Rust!");

    #[cfg(test)]
    test_main();

    unsafe {
        asm!("li t4, 0xFEEDFACECAFEBEEF");
    }

    loop {
        if let Some(c) = uart::UART.lock_with(|uart| uart.poll()) {
            println!("got char: '{}' (0x{:02X})", c as char, c);
            if c == b'q' {
                println!("shutting down");
                power::shutdown(power::ExitType::Success);
            } else if c == b'r' {
                println!("rebooting");
                power::shutdown(power::ExitType::Reboot);
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
