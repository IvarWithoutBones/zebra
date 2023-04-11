#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![no_std]

use bitbybit::{bitenum, bitfield};
use core::fmt;

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    tests.iter().for_each(|test| test());
}

// See docs/device-trees/qemu-virt.dts
pub const BASE_ADDR: u64 = 0x1000_0000;

pub trait UartRegister {
    fn ptr_offset() -> usize;
}

macro_rules! impl_uart_register {
    ($type:ty, $offset: expr) => {
        impl UartRegister for $type {
            #[inline]
            fn ptr_offset() -> usize {
                $offset
            }
        }
    };
}

#[bitfield(u8, default: 0)]
pub struct Interrupt {
    #[bit(0, rw)]
    enabled: bool,
}

impl_uart_register!(Interrupt, 1);

#[bitfield(u8, default: 0)]
pub struct Fifo {
    #[bit(0, rw)]
    enabled: bool,
}

impl_uart_register!(Fifo, 2);

#[allow(dead_code)]
#[bitenum(u2, exhaustive: true)]
pub enum WordLength {
    Five = 0,
    Six = 1,
    Seven = 2,
    Eight = 3,
}

#[bitfield(u8, default: 0)]
pub struct LineControl {
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
    base_ptr: u64,
    interrupt: Interrupt,
    fifo: Fifo,
    line_control: LineControl,
}

impl_uart_register!(NS16550a, 0);

impl NS16550a {
    pub const DEFAULT: NS16550a = NS16550a {
        base_ptr: BASE_ADDR,
        interrupt: Interrupt::new().with_enabled(true),
        fifo: Fifo::new().with_enabled(true),
        line_control: LineControl::new()
            .with_parity_enable(true)
            .with_word_length(WordLength::Eight),
    };

    pub const fn new(
        base_ptr: u64,
        interrupt: Interrupt,
        fifo: Fifo,
        line_control: LineControl,
    ) -> Self {
        Self {
            base_ptr,
            interrupt,
            fifo,
            line_control,
        }
    }

    pub fn init(&self) {
        self.write_register::<Interrupt>(self.interrupt.raw_value());
        self.write_register::<Fifo>(self.fifo.raw_value());
        self.write_register::<LineControl>(self.line_control.raw_value());
    }

    pub fn poll(&self) -> Option<u8> {
        let status = LineStatus::new_with_raw_value(self.read_register::<LineStatus>());
        if status.data_ready() {
            Some(self.read())
        } else {
            None
        }
    }

    pub fn read_register<T>(&self) -> u8
    where
        T: UartRegister,
    {
        let ptr = self.base_ptr as *mut u8;
        unsafe { ptr.add(T::ptr_offset()).read_volatile() }
    }

    pub fn write_register<T>(&self, data: u8)
    where
        T: UartRegister,
    {
        let ptr = self.base_ptr as *mut u8;
        unsafe { ptr.add(T::ptr_offset()).write_volatile(data) }
    }

    pub fn read(&self) -> u8 {
        self.read_register::<Self>()
    }

    pub fn write(&self, data: u8) {
        self.write_register::<Self>(data);
    }
}

impl fmt::Write for NS16550a {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        s.as_bytes().iter().for_each(|b| self.write(*b));
        Ok(())
    }
}
