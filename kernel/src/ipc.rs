use crate::spinlock::Spinlock;
use alloc::vec::Vec;

static SERVER_LIST: Spinlock<ServerList> = Spinlock::new(ServerList::new());

const MAX_MESSAGES: usize = 32;

#[derive(Debug, Copy, Clone)]
pub struct Message {
    pub pointer: *mut u8,
    pub length: usize,
}

#[derive(Debug)]
pub struct Server {
    pub process_id: usize,
    server_id: u128,

    pub messages: [Option<Message>; MAX_MESSAGES],
    current_message: usize,
}

impl Server {
    fn new(process_id: usize, server_id: u128) -> Self {
        Self {
            process_id,
            server_id,
            messages: [None; MAX_MESSAGES],
            current_message: 0,
        }
    }

    fn next_message(&mut self) {
        self.current_message = (self.current_message + 1) % MAX_MESSAGES;
    }

    pub fn send_message(&mut self, pointer: *mut u8, len: usize) {
        let message = Message { pointer, length: len };
        self.messages[self.current_message] = Some(message);
    }

    pub fn receive_message(&mut self) -> Option<Message> {
        let message = self.messages.get_mut(self.current_message)?.take();
        self.next_message();
        message
    }
}

#[derive(Debug)]
pub struct ServerList {
    servers: Vec<Server>,
}

impl ServerList {
    pub const fn new() -> Self {
        Self {
            servers: Vec::new(),
        }
    }

    pub fn register(&mut self, server_id: u128, process_id: usize) -> Option<()> {
        let server = Server::new(process_id, server_id);
        if self.servers.iter().any(|s| s.server_id == server_id) {
            return None;
        }

        self.servers.push(server);
        Some(())
    }

    pub fn get_by_pid(&mut self, process_id: usize) -> Option<&mut Server> {
        self.servers.iter_mut().find(|s| s.process_id == process_id)
    }

    pub fn get_by_sid(&mut self, server_id: u128) -> Option<&mut Server> {
        self.servers.iter_mut().find(|s| s.server_id == server_id)
    }
}

pub fn server_list() -> &'static Spinlock<ServerList> {
    &SERVER_LIST
}
