use crate::syscall;
use core::fmt::{self, Write};

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
        $crate::print::StandardOutput::print(true, format_args!($($args)+));
    }};

    () => {{
        $crate::print::StandardOutput::print(true, format_args!(""));
    }};
}

/// Printing helper to print to standard output, without a newline.
#[macro_export]
macro_rules! print {
    ($($args:tt)+) => {{
        $crate::print::StandardOutput::print(false, format_args!($($args)+));
    }};
}
