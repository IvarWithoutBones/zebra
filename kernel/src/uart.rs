use crate::spinlock::SpinLock;
use core::fmt::Write;

// Re-export the uart module from the crate shared with userland.
pub use uart::{NS16550a, BASE_ADDR};

pub static UART: SpinLock<uart::NS16550a> = SpinLock::new(NS16550a::DEFAULT);

/// Printing function that uses the UART to print to standard output.
pub fn print(with_newline: bool, args: ::core::fmt::Arguments) {
    UART.lock_with(|uart| {
        if with_newline {
            writeln!(uart, "{args}").unwrap();
        } else {
            write!(uart, "{args}").unwrap();
        }
    });
}

/// Printing helper that use the UART to print to standard output, with a newline.
#[macro_export]
macro_rules! println {
    ($($args:tt)+) => {{
        $crate::uart::print(true, format_args!($($args)+));
    }};

    () => {{
        $crate::uart::print(true, format_args!(""));
    }};
}

/// Printing helper that uses the UART to print to standard output.
#[macro_export]
macro_rules! print {
    ($($args:tt)+) => {{
        $crate::uart::print(false, format_args!($($args)+));
    }};
}
