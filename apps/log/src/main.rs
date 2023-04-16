#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]
#![no_main]

use core::sync::atomic::{AtomicU8, AtomicUsize, Ordering};
use librs::{
    ipc::{self, MessageData},
    syscall,
};
use log_server::{Reply, Request};

librs::main!(main);

/// A fixed size atomic queue for storing data in a circular buffer. Useful for saving data from interrupt handlers.
struct AtomicQueue<const LEN: usize> {
    read_index: AtomicUsize,
    write_index: AtomicUsize,
    buffer: [AtomicU8; LEN],
}

impl<const LEN: usize> AtomicQueue<LEN> {
    // An AtomicU8 is not Copy, because of which we cannot initialize an array of them without using a const.
    #[allow(clippy::declare_interior_mutable_const)] // We're never mutating this
    const BUFFER_ELEM_INIT: AtomicU8 = AtomicU8::new(u8::MAX);

    pub const fn new() -> Self {
        Self {
            read_index: AtomicUsize::new(0),
            write_index: AtomicUsize::new(0),
            buffer: [Self::BUFFER_ELEM_INIT; LEN],
        }
    }

    #[inline]
    fn to_index(&self, value: &AtomicUsize) -> usize {
        value.load(Ordering::Acquire) % LEN
    }

    pub fn push(&self, data: u8) {
        let write_index = self.to_index(&self.write_index);
        self.buffer[write_index].store(data, Ordering::Relaxed);
        self.write_index.fetch_add(1, Ordering::Relaxed);
    }

    pub fn pop(&self) -> Option<u8> {
        let read_index = self.to_index(&self.read_index);
        let write_index = self.to_index(&self.write_index);

        if read_index == write_index {
            None
        } else {
            self.read_index.fetch_add(1, Ordering::Relaxed);
            let value = self.buffer[read_index].load(Ordering::Relaxed);
            Some(value)
        }
    }
}

// NOTE: this is never initialized as that will be done by the kernel for debug purposes
static UART: uart::NS16550a = uart::NS16550a::DEFAULT;
static INPUT_QUEUE: AtomicQueue<32> = AtomicQueue::new();

fn main() {
    syscall::register_server(Some(u64::from_le_bytes(*b"log\0\0\0\0\0"))).unwrap();
    syscall::identity_map(uart::BASE_ADDR..=uart::BASE_ADDR + 0x1000);
    syscall::register_interrupt_handler(uart::INTERRUPT_ID, interrupt_handler);

    loop {
        let msg = ipc::Message::receive_blocking();
        let reply = ipc::MessageBuilder::new(msg.server_id);

        match Request::from(msg) {
            Request::Read => {
                if let Some(b) = INPUT_QUEUE.pop() {
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

// Will be called by the kernel when data is submitted to the UART
extern "C" fn interrupt_handler() {
    while let Some(b) = UART.poll() {
        INPUT_QUEUE.push(b);
    }

    syscall::complete_interrupt();
}
