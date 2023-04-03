#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]
#![no_main]

use librs::syscall;

librs::main!(main);

const UART_BASE: u64 = 0x1000_0000;

fn print(buf: &[u8]) {
    for b in buf {
        unsafe { (UART_BASE as *mut u8).write_volatile(*b) }
    }
}

fn main() {
    syscall::identity_map(UART_BASE..=UART_BASE + 1);
    syscall::register_server(b"log");

    loop {
        while let Some(msg) = syscall::receive_message() {
            print(msg);
        }

        syscall::wait_for_message();
    }
}
