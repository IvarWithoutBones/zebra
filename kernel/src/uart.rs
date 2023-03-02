use {
    crate::{power, spinlock::Spinlock, trap::plic::InterruptDevice},
    arbitrary_int::{u10, u3},
    bitbybit::{bitenum, bitfield},
    core::fmt::{self, Write},
};

// See device-trees/qemu-virt.dts
pub const BASE_ADDR: usize = 0x1000_0000;
pub const IRQ_ID: usize = 10;
pub static UART: Spinlock<NS16550a> = Spinlock::new(NS16550a::new(BASE_ADDR));

trait UartRegister {
    fn ptr_offset() -> usize;
}

macro_rules! impl_uart_register {
    ($type:ty, $offset: tt) => {
        impl UartRegister for $type {
            fn ptr_offset() -> usize {
                $offset
            }
        }
    };
}

#[bitfield(u8, default: 0)]
struct Interrupt {
    #[bit(0, rw)]
    enabled: bool,
}

impl_uart_register!(Interrupt, 1);

#[bitfield(u8, default: 0)]
struct Fifo {
    #[bit(0, rw)]
    enabled: bool,
}

impl_uart_register!(Fifo, 2);

#[allow(dead_code)]
#[bitenum(u2, exhaustive: true)]
enum WordLength {
    Five = 0,
    Six = 1,
    Seven = 2,
    Eight = 3,
}

#[bitfield(u8, default: 0)]
struct LineControl {
    #[bits(0..=1, rw)]
    word_length: WordLength,
    #[bit(3, rw)]
    parity_enable: bool,
}

impl_uart_register!(LineControl, 3);

#[bitfield(u8)]
struct LineStatus {
    #[bit(0, r)]
    data_ready: bool,
    #[bit(1, r)]
    overrun_error: bool,
    #[bit(2, r)]
    parity_error: bool,
    #[bit(3, r)]
    framing_error: bool,
}

impl_uart_register!(LineStatus, 5);

pub struct NS16550a {
    base_ptr: *mut u8,
    interrupt: Interrupt,
    fifo: Fifo,
    line_control: LineControl,
}

impl NS16550a {
    const fn new(base_addr: usize) -> Self {
        let interrupt = Interrupt::new().with_enabled(true);
        let fifo = Fifo::new().with_enabled(true);
        let line_control = LineControl::new()
            .with_parity_enable(true)
            .with_word_length(WordLength::Eight);

        Self {
            base_ptr: base_addr as _,
            interrupt,
            fifo,
            line_control,
        }
    }

    pub fn init(&self) {
        self.write::<Interrupt>(self.interrupt.raw_value());
        self.write::<Fifo>(self.fifo.raw_value());
        self.write::<LineControl>(self.line_control.raw_value());
    }

    pub fn poll(&self) -> Option<u8> {
        let status = LineStatus::new_with_raw_value(self.read::<LineStatus>());
        if status.data_ready() {
            Some(self.read::<Self>())
        } else {
            None
        }
    }

    fn read<T>(&self) -> u8
    where
        T: UartRegister,
    {
        unsafe { self.base_ptr.add(T::ptr_offset()).read_volatile() }
    }

    fn write<T>(&self, data: u8)
    where
        T: UartRegister,
    {
        unsafe { self.base_ptr.add(T::ptr_offset()).write_volatile(data) }
    }
}

impl UartRegister for NS16550a {
    fn ptr_offset() -> usize {
        0
    }
}

impl fmt::Write for NS16550a {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &byte in s.as_bytes() {
            self.write::<Self>(byte);
        }
        Ok(())
    }
}

impl InterruptDevice for NS16550a {
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
