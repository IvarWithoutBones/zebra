#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]
#![no_main]

use librs::{ipc, syscall};
use log_server::{Reply, Request};

librs::main!(main);

// NOTE: this is never initialized as that will be done by the kernel for debug purposes
static UART: uart::NS16550a = uart::NS16550a::DEFAULT;

fn main() {
    syscall::register_server(Some(u64::from_le_bytes(*b"log\0\0\0\0\0"))).unwrap();
    syscall::identity_map(uart::BASE_ADDR..=uart::BASE_ADDR + 5);

    loop {
        let msg = ipc::Message::receive_blocking();
        let reply = ipc::MessageBuilder::new(msg.server_id);

        match Request::from(msg) {
            Request::Read => {
                if let Some(b) = UART.poll() {
                    reply
                        .with_identifier(Reply::DataReady { data: b }.to_identifier())
                        .with_data(b as u64)
                        .send();
                } else {
                    reply
                        .with_identifier(Reply::DataNotReady.to_identifier())
                        .send();
                }
            }

            Request::Write { data } => {
                data.to_be_bytes().iter().for_each(|b| UART.write_byte(*b));
            }

            Request::Unknown { id } => {
                reply
                    .with_identifier(Reply::RequestUnknown.to_identifier())
                    .with_data(id)
                    .send();
            }
        }
    }
}
