use {
    tartan_bitfield::bitfield,
    tartan_c_enum::c_enum,
    {super::spinlock::Spinlock, core::fmt},
};

// See device-trees/qemu-virt.dts
pub static UART: Spinlock<NS16550a<0x1000_0000>> = Spinlock::new(NS16550a::new());

trait UartRegister {
    fn ptr_offset() -> usize;
}

bitfield! {
    struct Interrupt(u8) {
        [0] enabled,
    }
}

impl UartRegister for Interrupt {
    fn ptr_offset() -> usize {
        1
    }
}

bitfield! {
    struct Fifo(u8) {
        [0] enabled,
    }
}

impl UartRegister for Fifo {
    fn ptr_offset() -> usize {
        2
    }
}

c_enum! {
    enum WordLength(u8) {
        Five = 0,
        Six = 1,
        Seven = 2,
        Eight = 3,
    }
}

bitfield! {
    struct LineControl(u8) {
        [0..=2] word_length: u8 as WordLength,
        [3] parity_enable,
    }
}

impl UartRegister for LineControl {
    fn ptr_offset() -> usize {
        3
    }
}

bitfield! {
    struct LineStatus(u8) {
        [0] data_ready,
        [1] overrun_error,
        [2] parity_error,
        [3] framing_error,
    }
}

impl UartRegister for LineStatus {
    fn ptr_offset() -> usize {
        5
    }
}

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
        let interrupt = Interrupt { 0: 0 };
        let line_control = LineControl { 0: 0 };
        let fifo = Fifo { 0: 0 };

        Self {
            interrupt,
            fifo,
            line_control,
        }
    }

    pub fn init(&mut self) {
        self.interrupt.set_enabled(true);
        self.fifo.set_enabled(true);
        self.line_control.set_word_length(WordLength::Eight);
        self.line_control.set_parity_enable(false);

        self.write_register(self.interrupt);
        self.write_register(self.fifo);
        self.write_register(self.line_control);
    }

    pub fn poll(&self) -> Option<u8> {
        let status = self.read_register::<LineStatus>();
        if status.data_ready() {
            Some(self.read())
        } else {
            None
        }
    }

    fn read(&self) -> u8 {
        unsafe { Self::BASE_PTR.read_volatile() }
    }

    fn write(&self, data: u8) {
        unsafe { Self::BASE_PTR.write_volatile(data) }
    }

    fn read_register<T>(&self) -> T
    where
        T: UartRegister + From<u8>,
    {
        T::from(unsafe { Self::BASE_PTR.add(T::ptr_offset()).read_volatile() })
    }

    fn write_register<T>(&self, data: T)
    where
        T: UartRegister + Into<u8>,
    {
        unsafe {
            Self::BASE_PTR
                .add(T::ptr_offset())
                .write_volatile(data.into())
        }
    }
}

impl<const BASE_ADDR: usize> fmt::Write for NS16550a<BASE_ADDR> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for &byte in s.as_bytes() {
            self.write(byte);
        }
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($args:tt)+) => {{
        use core::fmt::Write;
        $crate::uart::UART.lock_with(|uart| {
            let _ = write!(uart, $($args)+);
        })
    }};
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\r\n")
    };

    ($fmt:expr) => ({
		$crate::print!(concat!($fmt, "\r\n"))
	});

    ($fmt:expr, $($args:tt)+) => ({
		print!(concat!($fmt, "\r\n"), $($args)+)
	});
}
