use crate::syscall;

use core::{
    fmt::{self, Write},
    panic::PanicInfo,
};

pub struct StandardOutput;

impl Write for StandardOutput {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        syscall::print(s);
        Ok(())
    }
}

impl StandardOutput {
    pub fn print(with_newline: bool, args: ::core::fmt::Arguments) {
        if with_newline {
            writeln!(StandardOutput, "{args}").unwrap();
        } else {
            write!(StandardOutput, "{args}").unwrap();
        }
    }
}

/// Printing helper to print to standard output, with a newline.
#[macro_export]
macro_rules! println {
    ($($args:tt)+) => {{
        $crate::language_items::StandardOutput::print(true, format_args!($($args)+));
    }};

    () => {{
        $crate::language_items::StandardOutput::print(true, format_args!(""));
    }};
}

/// Printing helper to print to standard output, without a newline.
#[macro_export]
macro_rules! print {
    ($($args:tt)+) => {{
        $crate::language_items::StandardOutput::print(false, format_args!($($args)+));
    }};
}

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
        print!("{}...\t", core::any::type_name::<T>());
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
