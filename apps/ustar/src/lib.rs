#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]
#![no_main]

extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec::Vec,
};
use bitbybit::bitenum;
use librs::ipc;

pub const SID: u64 = u64::from_be_bytes(*b"ustar\0\0\0");

pub type FileIndex = usize;

#[bitenum(u64, exhaustive: false)]
#[derive(Debug)]
#[repr(u64)]
pub enum Request {
    ListFiles = 1,
    FileName = 4,
    FileIndex = 5,
    FileContents = 6,
    UnknownRequest = 0xffff,
}

impl From<&ipc::Message> for Request {
    fn from(value: &ipc::Message) -> Self {
        Self::new_with_raw_value(value.identifier).unwrap_or(Request::UnknownRequest)
    }
}

impl From<Request> for u64 {
    fn from(val: Request) -> Self {
        val.raw_value()
    }
}

#[bitenum(u64, exhaustive: false)]
#[derive(Debug)]
#[repr(u64)]
pub enum Reply {
    ReplyCount = 2,
    FileName = 4,
    FileIndex = 5,
    FileContents = 6,
    IsDirectory = 7,
    NoChildren = 0xfffe,
    FileNotFound = 0xfffd,
    UnknownRequest = 0xffff,
}

impl From<&ipc::Message> for Reply {
    fn from(value: &ipc::Message) -> Self {
        Self::new_with_raw_value(value.identifier).unwrap_or(Reply::UnknownRequest)
    }
}

impl From<Reply> for u64 {
    fn from(val: Reply) -> Self {
        val.raw_value()
    }
}

pub fn file_index_of(path: &str) -> Result<FileIndex, Reply> {
    let reply = ipc::MessageBuilder::new(SID)
        .with_identifier(Request::FileIndex.into())
        .with_data(path.into())
        .build()
        .send_receive()
        .unwrap();

    if reply.identifier == Reply::FileIndex.into() {
        Ok(reply.data[0] as _)
    } else {
        Err(Reply::from(&reply))
    }
}

pub fn file_name(file_id: FileIndex) -> Result<String, Reply> {
    let reply = ipc::MessageBuilder::new(SID)
        .with_identifier(Request::FileName.into())
        .with_data((file_id as u64).into())
        .build()
        .send_receive()
        .unwrap();

    if reply.identifier == Reply::FileName.into() {
        let bytes = reply.data.as_be_bytes();
        Ok(core::str::from_utf8(&bytes).unwrap().to_string())
    } else {
        Err(Reply::from(&reply))
    }
}

pub fn file_name_of_path(path: &str) -> Result<String, Reply> {
    let file_id = file_index_of(path)?;
    file_name(file_id)
}

pub fn read_file(file_id: FileIndex) -> Result<Vec<u8>, Reply> {
    let reply = ipc::MessageBuilder::new(SID)
        .with_identifier(Request::FileContents.into())
        .with_data((file_id as u64).into())
        .build()
        .send_receive()
        .unwrap();

    if reply.identifier != Reply::FileContents.into() {
        return Err(Reply::from(&reply));
    }

    let ptr = reply.data[0] as *mut u8;
    let len = reply.data[1] as usize;
    Ok(unsafe { Vec::from_raw_parts(ptr, len, len) })
}

pub fn read_file_from_path(path: &str) -> Result<Vec<u8>, Reply> {
    let file_id = file_index_of(path)?;
    read_file(file_id)
}

pub fn children(parent: FileIndex) -> Result<Vec<FileIndex>, Reply> {
    let reply = ipc::MessageBuilder::new(SID)
        .with_identifier(Request::ListFiles.into())
        .with_data((parent as u64).into())
        .build()
        .send_receive()
        .unwrap();

    if reply.identifier != Reply::ReplyCount.into() {
        return Err(Reply::from(&reply));
    }

    let length = reply.data[0];
    let mut files = Vec::with_capacity(length as usize);

    for _ in 0..length {
        let reply = ipc::Message::receive_blocking();
        if reply.identifier != Reply::FileIndex.into() {
            return Err(Reply::from(&reply));
        }

        for id in reply
            .data
            .iter()
            .take_while(|&&id| id != Reply::NoChildren.into())
        {
            files.push(*id as _);
        }
    }

    Ok(files)
}

pub fn children_of_path(path: &str) -> Result<Vec<FileIndex>, Reply> {
    let file_id = file_index_of(path)?;
    children(file_id)
}
