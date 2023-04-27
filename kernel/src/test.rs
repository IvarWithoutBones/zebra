//! Integration for Rust language features that are not implemented by default on `no_std`.

#[cfg(test)]
use core::any::type_name;
use crate::power;
use core::{arch::asm, panic::PanicInfo};

/// A wrapper around `Fn()` which can be used as a trait object,
/// used to allow fetching the name of the test function.
#[cfg(test)]
pub trait Testable {
    fn run(&self);
}

#[cfg(test)]
impl<T> Testable for T
where
    T: Fn(),
{
    fn run(&self) {
        print!("{}...\t", type_name::<T>());
        self();
        println!("[ok]");
    }
}

/// The entry point for `cargo test`, called by the `test_runner` exported in `main.rs`.
#[cfg(test)]
pub fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test.run()
    }
    power::shutdown(power::ExitType::Success)
}

/// Called on panic, prints the panic message and shuts down the system.
/// Note that this only covers panics from Rust itself, not CPU exceptions.
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    // Disable interrupts, otherwise we might never get to shut down
    unsafe { asm!("csrw sie, zero") }

    println!("{:#}", info);
    power::shutdown(power::ExitType::Failure)
}
