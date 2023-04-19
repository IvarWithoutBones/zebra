#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]
#![no_main]

use bitbybit::bitenum;
use librs::ipc::{self, MessageData};

pub const SERVER_ID: u64 = 123;

#[bitenum(u64, exhaustive: false)]
#[derive(Debug)]
pub enum Request {
    ReadDisk = 1,
    DiskSize = 2,
    UnknownRequest = 0xff,
}

impl Request {
    pub fn to_message(&self, data: MessageData) -> ipc::Message {
        ipc::Message {
            identifier: self.raw_value(),
            server_id: SERVER_ID,
            data,
        }
    }
}

impl From<Request> for ipc::Message {
    fn from(val: Request) -> Self {
        val.to_message(MessageData::default())
    }
}

impl From<&ipc::Message> for Request {
    fn from(value: &ipc::Message) -> Self {
        Self::new_with_raw_value(value.identifier).unwrap_or(Request::UnknownRequest)
    }
}

#[bitenum(u64, exhaustive: false)]
#[derive(Debug, PartialEq, Eq)]
pub enum Reply {
    DataReady = 1,
    DiskSize = 2,
    UnknownRequest = 0xff,
}

impl Reply {
    pub fn from_message(msg: &ipc::Message) -> Option<Self> {
        Self::new_with_raw_value(msg.identifier).ok()
    }

    pub fn to_message(&self, server_id: u64, data: MessageData) -> ipc::Message {
        ipc::Message {
            identifier: self.raw_value(),
            server_id,
            data,
        }
    }
}

/// # Safety
/// The caller must ensure that the message contains valid data.
pub unsafe fn reply_as_slice(msg: &ipc::Message) -> Option<&[u8]> {
    if Reply::from_message(msg) != Some(Reply::DataReady) {
        return None;
    }

    Some(core::slice::from_raw_parts(
        msg.data[0] as *const u8,
        msg.data[1] as usize,
    ))
}
