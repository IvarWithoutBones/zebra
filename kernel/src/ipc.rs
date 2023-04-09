use crate::spinlock::Spinlock;
use alloc::{collections::VecDeque, vec::Vec};
use core::sync::atomic::{AtomicU64, Ordering};

static SERVER_LIST: Spinlock<ServerList> = Spinlock::new(ServerList::new());
static NEXT_SERVER_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub identifier: u64,
    pub data: u64,
    pub sender_pid: usize,
    pub sender_sid: u64,
}

impl Message {
    pub const fn new(
        sender_pid: usize,
        sender_sid: u64,
        identifier: u64,
        data: u64,
    ) -> Self {
        Self {
            sender_sid,
            sender_pid,
            identifier,
            data,
        }
    }
}

#[derive(Debug)]
pub struct Server {
    pub process_id: usize,
    pub server_id: u64,
    messages: VecDeque<Message>,
}

impl Server {
    fn new(process_id: usize, public_name: Option<u64>) -> Self {
        let server_id =
            public_name.unwrap_or_else(|| NEXT_SERVER_ID.fetch_add(1, Ordering::SeqCst));

        Self {
            process_id,
            server_id,
            messages: VecDeque::new(),
        }
    }

    pub fn has_messages(&self) -> bool {
        !self.messages.is_empty()
    }

    pub fn send_message(&mut self, message: Message) {
        self.messages.push_back(message);
    }

    pub fn receive_message(&mut self) -> Option<Message> {
        self.messages.pop_front()
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

    pub fn register(&mut self, process_id: usize, server_id: Option<u64>) -> Option<u64> {
        if self.servers.iter().any(|s| s.process_id == process_id) {
            return None;
        }

        if let Some(server_id) = server_id {
            if self.servers.iter().any(|s| s.server_id == server_id) {
                return None;
            }
        }

        let server = Server::new(process_id, server_id);
        let server_id = server.server_id;
        self.servers.push(server);
        Some(server_id)
    }

    pub fn get_by_pid(&mut self, process_id: usize) -> Option<&mut Server> {
        self.servers.iter_mut().find(|s| s.process_id == process_id)
    }

    pub fn get_by_sid(&mut self, server_id: u64) -> Option<&mut Server> {
        self.servers.iter_mut().find(|s| s.server_id == server_id)
    }
}

pub fn server_list() -> &'static Spinlock<ServerList> {
    &SERVER_LIST
}
