#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]

use librs::ipc::{self, MessageData};

const SERVER_ID: u64 = u64::from_ne_bytes(*b"log\0\0\0\0\0");

#[derive(Debug, Clone, Copy)]
pub enum Request {
    Read,
    Write { data: MessageData },
    Unknown { id: u64 },
}

impl Request {
    pub const fn to_message(self) -> ipc::Message {
        let msg = ipc::MessageBuilder::new(SERVER_ID);

        match self {
            Request::Read => msg.with_identifier(2),
            Request::Write { data } => msg.with_identifier(1).with_data(data),
            Request::Unknown { id } => msg.with_identifier(id),
        }
        .build()
    }
}

impl From<ipc::Message> for Request {
    fn from(msg: ipc::Message) -> Request {
        match msg.identifier {
            2 => Request::Read,
            1 => Request::Write { data: msg.data },
            _ => Request::Unknown { id: msg.identifier },
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Reply {
    DataNotReady,
    DataReady { data: MessageData },
    RequestUnknown,
}

impl From<ipc::Message> for Reply {
    fn from(msg: ipc::Message) -> Reply {
        match msg.identifier {
            0 => Reply::DataNotReady,
            1 => Reply::DataReady { data: msg.data },
            _ => Reply::RequestUnknown,
        }
    }
}

impl Reply {
    pub const fn to_identifier(self) -> u64 {
        match self {
            Reply::DataNotReady => 0,
            Reply::DataReady { .. } => 1,
            Reply::RequestUnknown => 2,
        }
    }
}

pub fn read() -> Option<u8> {
    if let Some(msg) = Request::Read.to_message().send_receive() {
        let reply = Reply::from(msg);
        match reply {
            // TODO: reply with more data if available
            Reply::DataReady { data } => Some(data[0] as _),
            _ => None,
        }
    } else {
        None
    }
}

pub fn write(data: MessageData) {
    Request::Write { data }.to_message().send();
}
