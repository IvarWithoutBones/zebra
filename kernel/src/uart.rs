use crate::{power, spinlock::SpinLock, trap::plic::InterruptDevice};
use core::fmt::Write;

// Re-export the uart module from the crate shared with userland.
pub use uart::{NS16550a, BASE_ADDR};

pub static UART: SpinLock<uart::NS16550a> = SpinLock::new(NS16550a::DEFAULT);

impl InterruptDevice for SpinLock<uart::NS16550a> {
    fn identifier(&self) -> u16 {
        10
    }

    fn priority(&self) -> u8 {
        1
    }

    fn handle() {
        // TODO: blocking here is unfortunate, this should be moved to a queue instead.
        UART.lock_with(|uart| {
            while let Some(byte) = uart.poll() {
                let c = byte as char;
                writeln!(uart, "got char: '{c}' ({byte:#02x})").unwrap();

                match c {
                    'q' => {
                        writeln!(uart, "shutting down").unwrap();
                        power::shutdown(power::ExitType::Success);
                    }

                    'r' => {
                        writeln!(uart, "rebooting").unwrap();
                        power::shutdown(power::ExitType::Reboot);
                    }

                    _ => {}
                }
            }
        });
    }
}
