use {
    crate::{power, spinlock::Spinlock, trap::plic::InterruptDevice},
    bitbybit::{bitenum, bitfield},
    core::fmt::{self, Write},
};

// See device-trees/qemu-virt.dts
pub const BASE_ADDR: usize = 0x1000_0000;
pub const IRQ_ID: usize = 10;
pub static UART: Spinlock<NS16550a<BASE_ADDR>> = Spinlock::new(NS16550a::new());

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

pub struct NS16550a<const BASE_ADDR: usize> {
    interrupt: Interrupt,
    fifo: Fifo,
    line_control: LineControl,
}

impl<const BASE_ADDR: usize> UartRegister for NS16550a<BASE_ADDR> {
    fn ptr_offset() -> usize {
        0
    }
}

impl<const BASE_ADDR: usize> NS16550a<BASE_ADDR> {
    const BASE_PTR: *mut u8 = BASE_ADDR as _;

    const fn new() -> Self {
        let interrupt = Interrupt::new().with_enabled(true);
        let fifo = Fifo::new().with_enabled(true);
        let line_control = LineControl::new()
            .with_parity_enable(true)
            .with_word_length(WordLength::Eight);

        Self {
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
        unsafe { Self::BASE_PTR.add(T::ptr_offset()).read_volatile() }
    }

    fn write<T>(&self, data: u8)
    where
        T: UartRegister,
    {
        unsafe { Self::BASE_PTR.add(T::ptr_offset()).write_volatile(data) }
    }
}

impl<const BASE_ADDR: usize> fmt::Write for NS16550a<BASE_ADDR> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &byte in s.as_bytes() {
            self.write::<Self>(byte);
        }
        Ok(())
    }
}

impl<const BASE_ADDR: usize> InterruptDevice for NS16550a<BASE_ADDR> {
    const INTERRUPT_ID: arbitrary_int::u10 = arbitrary_int::u10::new(10);

    fn priority() -> arbitrary_int::u3 {
        arbitrary_int::u3::new(1)
    }
}

pub fn interrupt() {
    UART.lock_with(|uart| {
        while let Some(byte) = uart.poll() {
            let c = byte as char;
            write!(uart, "got char: '{c}' ({byte:#02x})\n").unwrap();

            match c {
                'q' => {
                    write!(uart, "shutting down\n").unwrap();
                    power::shutdown(power::ExitType::Success);
                }

                'r' => {
                    write!(uart, "rebooting\n").unwrap();
                    power::shutdown(power::ExitType::Reboot);
                }

                _ => {}
            }
        }
    });
}
