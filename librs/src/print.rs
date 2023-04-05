use crate::syscall;
use core::{
    fmt::{self, Write},
    mem::size_of,
};

pub struct StandardOutput;

impl Write for StandardOutput {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        s.as_bytes().chunks(size_of::<u64>()).for_each(|chunk| {
            let mut buf = [0; size_of::<u64>()];
            buf[..chunk.len()].copy_from_slice(chunk);

            syscall::send_message(
                u64::from_ne_bytes(*b"log\0\0\0\0\0"),
                1, // Print
                u64::from_be_bytes(buf),
            );
        });
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
