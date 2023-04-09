use crate::syscall;

#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub server_id: u64,
    pub identifier: u64,
    pub data: u64,
}

impl Message {
    pub const fn new(server_id: u64, identifier: u64, data: u64) -> Message {
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
        syscall::receive_message().map(|(identifier, data, server_id)| Message {
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
    data: u64,
}

impl MessageBuilder {
    pub const fn new(server_id: u64) -> MessageBuilder {
        Self {
            server_id,
            identifier: 0,
            data: 0,
        }
    }

    pub const fn with_identifier(mut self, identifier: u64) -> MessageBuilder {
        self.identifier = identifier;
        self
    }

    pub const fn with_data(mut self, data: u64) -> MessageBuilder {
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
