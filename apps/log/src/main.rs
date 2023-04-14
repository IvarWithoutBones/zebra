#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]
#![no_main]

use alloc::collections::VecDeque;
use librs::{
    ipc::{self, MessageData},
    mutex::Mutex,
    syscall,
};
use log_server::{Reply, Request};

librs::main!(main);

// NOTE: this is never initialized as that will be done by the kernel for debug purposes
static UART: uart::NS16550a = uart::NS16550a::DEFAULT;
static INPUT_BUFFER: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());

fn main() {
    syscall::register_server(Some(u64::from_le_bytes(*b"log\0\0\0\0\0"))).unwrap();
    syscall::identity_map(uart::BASE_ADDR..=uart::BASE_ADDR + 0x1000);
    syscall::register_interrupt_handler(10, interrupt_handler);

    loop {
        let msg = ipc::Message::receive_blocking();
        let reply = ipc::MessageBuilder::new(msg.server_id);

        match Request::from(msg) {
            Request::Read => {
                if let Some(b) = INPUT_BUFFER.lock().pop_front() {
                    // TODO: reply with more data if available
                    let data = MessageData::from(b as u64);

                    reply
                        .with_identifier(Reply::DataReady { data }.to_identifier())
                        .with_data(data)
                        .send();
                } else {
                    reply
                        .with_identifier(Reply::DataNotReady.to_identifier())
                        .send();
                }
            }

            Request::Write { data } => {
                data.iter()
                    .for_each(|num| num.to_be_bytes().iter().for_each(|b| UART.write(*b)));
            }

            Request::Unknown { id } => {
                reply
                    .with_identifier(Reply::RequestUnknown.to_identifier())
                    .with_data(id.into())
                    .send();
            }
        }
    }
}

extern "C" fn interrupt_handler() {
    let mut buf = INPUT_BUFFER.lock();
    while let Some(b) = UART.poll() {
        buf.push_back(b);
    }

    // TODO: how do we run destructors prior to the kernel taking over?
    drop(buf);
    syscall::complete_interrupt();
}
