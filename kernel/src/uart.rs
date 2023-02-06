use packed_struct::prelude::*;

// See device-trees/qemu-virt.dts
pub static UART: NS16550a<0x1000_0000> = NS16550a::new();

trait UartRegister {
    fn ptr_offset() -> usize;
}

#[derive(PackedStruct)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "1")]
struct Interrupt {
    #[packed_field(bits = "0")]
    enable: bool,
}

impl UartRegister for Interrupt {
    fn ptr_offset() -> usize {
        1
    }
}

#[derive(PackedStruct)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "1")]
struct Fifo {
    #[packed_field(bits = "0")]
    enable: bool,
}

impl UartRegister for Fifo {
    fn ptr_offset() -> usize {
        2
    }
}

#[derive(PrimitiveEnum_u8, Copy, Clone)]
enum WordLength {
    Five = 0,
    Six = 1,
    Seven = 2,
    Eight = 3,
}

#[derive(PackedStruct)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "1")]
struct LineControl {
    #[packed_field(bits = "0..=2", ty = "enum")]
    word_length: WordLength,
    #[packed_field(bits = "3")]
    parity_enable: bool,
}

impl UartRegister for LineControl {
    fn ptr_offset() -> usize {
        3
    }
}

#[derive(PackedStruct)]
#[packed_struct(bit_numbering = "lsb0", size_bytes = "1")]
struct LineStatus {
    #[packed_field(bits = "0")]
    data_ready: bool,
    #[packed_field(bits = "1")]
    overrun_error: bool,
    #[packed_field(bits = "2")]
    parity_error: bool,
    #[packed_field(bits = "3")]
    framing_error: bool,
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
        Self {
            interrupt: Interrupt { enable: true },
            fifo: Fifo { enable: true },
            line_control: LineControl {
                word_length: WordLength::Eight,
                parity_enable: false,
            },
        }
    }

    pub fn init(&self) {
        self.write::<Interrupt>(self.interrupt.pack().unwrap()[0]);
        self.write::<Fifo>(self.fifo.pack().unwrap()[0]);
        self.write::<LineControl>(self.line_control.pack().unwrap()[0]);
    }

    pub fn poll(&self) -> Option<u8> {
        let status = LineStatus::unpack(&[self.read::<LineStatus>()]).ok()?;
        if status.data_ready {
            Some(self.read::<Self>())
        } else {
            None
        }
    }

    pub fn print(&self, msg: &str) {
        for &byte in msg.as_bytes() {
            self.write::<Self>(byte);
        }
    }

    pub fn print_num(&self, num: usize, radix: usize) {
        let tens = num / radix;
        if tens > 0 {
            self.print_num(tens, radix);
        }

        let c = (num % radix) as u8 + if (num % radix) < 10 { b'0' } else { b'a' - 10 };
        self.write::<Self>(c);
    }

    fn read<T: UartRegister>(&self) -> u8 {
        unsafe { Self::BASE_PTR.add(T::ptr_offset()).read_volatile() }
    }

    fn write<T: UartRegister>(&self, data: u8) {
        unsafe { Self::BASE_PTR.add(T::ptr_offset()).write_volatile(data) }
    }
}

// Extremely barebones printing macros without any (non-static) formatting support,
// when an allocator is implemented this can be replaced with something proper.

#[macro_export]
macro_rules! print {
    ($fmt: expr) => ($crate::uart::UART.print(&$fmt));
    ($($arg:expr),*) => {{
        $($crate::uart::UART.print(&$arg);)*
    }};
}

#[macro_export]
macro_rules! println {
    () => {
        $crate::print!("\r\n")
    };

    ($($args:expr),*) => ({
		$crate::print!($($args),*);
        $crate::print!("\r\n");
	});
}

#[macro_export]
macro_rules! print_num {
    ($num: expr) => {
        $crate::uart::UART.print_num($num, 10)
    };

    ($num:expr, $radix:expr) => {
        $crate::uart::UART.print_num($num, $radix)
    };
}

#[macro_export]
macro_rules! println_num {
    ($num: expr) => {{
        $crate::print_num!($num);
        $crate::println!();
    }};

    ($num:expr, $radix:expr) => {{
        $crate::print_num!($num, $radix);
        $crate::println!();
    }};
}
