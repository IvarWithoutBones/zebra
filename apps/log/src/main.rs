#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]
#![no_main]

use librs::{ipc, syscall};

librs::main!(main);

const ID_WRITE: u64 = 1;
const ID_READ: u64 = 2;

const REPLY_DATA_NOT_READY: u64 = 0;
const REPLY_DATA_READY: u64 = 1;
const REPLY_REQUEST_UNKNOWN: u64 = u64::MAX;

const UART_BASE: u64 = 0x1000_0000;

fn print(buf: &[u8]) {
    for b in buf {
        unsafe { (UART_BASE as *mut u8).write_volatile(*b) }
    }
}

fn read() -> Option<u8> {
    let status = unsafe { (UART_BASE as *mut u8).add(5).read_volatile() };
    if status & 1 == 1 {
        Some(unsafe { (UART_BASE as *mut u8).read_volatile() })
    } else {
        None
    }
}

fn main() {
    syscall::register_server(Some(u64::from_le_bytes(*b"log\0\0\0\0\0"))).unwrap();
    syscall::identity_map(UART_BASE..=UART_BASE + 5);

    loop {
        let msg = ipc::Message::receive_blocking();
        let reply = ipc::MessageBuilder::new(msg.server_id);

        match msg.identifier {
            ID_WRITE => print(&msg.data.to_be_bytes()),

            ID_READ => {
                if let Some(b) = read() {
                    reply
                        .with_identifier(REPLY_DATA_READY)
                        .with_data(b as u64)
                        .send();
                } else {
                    reply.with_identifier(REPLY_DATA_NOT_READY).send();
                }
            }

            _ => reply.with_identifier(REPLY_REQUEST_UNKNOWN).send(),
        }
    }
}
