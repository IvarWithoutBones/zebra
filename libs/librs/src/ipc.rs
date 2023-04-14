use crate::syscall;
use core::{
    mem::size_of,
    ops::{Index, IndexMut},
};

// TODO: merge with kernel
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[repr(transparent)]
pub struct MessageData {
    pub data: [u64; 5],
}

impl MessageData {
    pub const DEFAULT: MessageData = MessageData { data: [0; 5] };
    pub const LEN: usize = 5;

    pub const fn new(data: [u64; 5]) -> MessageData {
        MessageData { data }
    }

    pub const fn from_u64(data: u64) -> MessageData {
        MessageData {
            data: [data, 0, 0, 0, 0],
        }
    }

    pub const fn as_slice(&self) -> &[u64] {
        &self.data
    }

    pub fn iter(&self) -> impl Iterator<Item = &u64> {
        self.data.iter()
    }
}

impl Index<usize> for MessageData {
    type Output = u64;

    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index]
    }
}

impl IndexMut<usize> for MessageData {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index]
    }
}

impl From<u64> for MessageData {
    fn from(data: u64) -> Self {
        Self::from_u64(data)
    }
}

impl From<[u64; 5]> for MessageData {
    fn from(data: [u64; 5]) -> Self {
        Self::new(data)
    }
}

impl From<&[u64]> for MessageData {
    fn from(data: &[u64]) -> Self {
        let mut result = Self::DEFAULT;
        result.data[..data.len()].copy_from_slice(data);
        result
    }
}

impl From<&[u8]> for MessageData {
    fn from(data: &[u8]) -> Self {
        let mut result = Self::DEFAULT;
        data.chunks(size_of::<u64>())
            .enumerate()
            .for_each(|(i, chunk)| {
                let mut buf = [0; size_of::<u64>()];
                buf[..chunk.len()].copy_from_slice(chunk);
                result.data[i] = u64::from_be_bytes(buf);
            });
        result
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub server_id: u64,
    pub identifier: u64,
    pub data: MessageData,
}

impl Message {
    pub const fn new(server_id: u64, identifier: u64, data: MessageData) -> Message {
        Self {
            server_id,
            identifier,
            data,
        }
    }

    pub fn send(self) {
        syscall::send_message(self.server_id, self.identifier, self.data);
    }

    pub fn receive() -> Option<Message> {
        syscall::receive_message().map(|(identifier, server_id, data)| Message {
            server_id,
            identifier,
            data,
        })
    }

    pub fn receive_blocking() -> Message {
        loop {
            if let Some(msg) = Self::receive() {
                return msg;
            } else {
                syscall::wait_until_message_received();
            }
        }
    }

    pub fn send_receive(self) -> Option<Message> {
        syscall::send_message(self.server_id, self.identifier, self.data);
        syscall::wait_until_message_received();
        // TODO: ensure that the message came from the server we contacted
        Message::receive()
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct MessageBuilder {
    server_id: u64,
    identifier: u64,
    data: MessageData,
}

impl MessageBuilder {
    pub const fn new(server_id: u64) -> MessageBuilder {
        Self {
            server_id,
            identifier: 0,
            data: MessageData::DEFAULT,
        }
    }

    pub const fn with_identifier(mut self, identifier: u64) -> MessageBuilder {
        self.identifier = identifier;
        self
    }

    pub const fn with_data(mut self, data: MessageData) -> MessageBuilder {
        self.data = data;
        self
    }

    pub const fn build(self) -> Message {
        Message {
            server_id: self.server_id,
            identifier: self.identifier,
            data: self.data,
        }
    }

    pub fn send(self) {
        self.build().send();
    }
}
