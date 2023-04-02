use crate::syscall;
use core::{any::type_name, panic::PanicInfo};

// TODO: this module does not work at all with `cargo test` yet, as executables need to be executed from the kernel.

/// A wrapper around `Fn()` which can be used as a trait object,
/// used to allow fetching the name of the test function.
pub trait Testable {
    fn run(&self);
}

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
pub fn test_runner(tests: &[&dyn Testable]) {
    println!("Running {} tests", tests.len());
    for test in tests {
        test.run()
    }
    syscall::exit();
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    println!("{info}");
    syscall::exit();
}
