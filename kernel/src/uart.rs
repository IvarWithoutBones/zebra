use crate::{power, spinlock::Spinlock, trap::plic::InterruptDevice};
use arbitrary_int::{u10, u3};
use core::fmt::Write;

// Re-export the uart module from the crate shared with userland.
pub use uart::{NS16550a, BASE_ADDR};

pub const IRQ_ID: usize = 10;
pub static UART: Spinlock<uart::NS16550a> = Spinlock::new(NS16550a::DEFAULT);

impl InterruptDevice for uart::NS16550a {
    const INTERRUPT_ID: u10 = u10::new(10);

    fn priority() -> u3 {
        arbitrary_int::u3::new(1)
    }
}

pub fn interrupt() {
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
