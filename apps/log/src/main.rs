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
    syscall::register_server(Some(u64::from_ne_bytes(*b"log\0\0\0\0\0"))).unwrap();
    syscall::identity_map(UART_BASE..=UART_BASE + 1);

    loop {
        while let Some(msg) = syscall::receive_message() {
            if msg.0 == 1 {
                let buf = msg.1.to_be_bytes();
                print(&buf);
            }
        }

        syscall::wait_until_message_received();
    }
}
