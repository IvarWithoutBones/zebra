#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]
#![no_main]

use alloc::{fmt, vec::Vec};
use binrw::{binrw, BinRead, BinReaderExt, NullString};
use core::{ops::Index, str};
use librs::ipc;
use ustar::{FileIndex, Reply, Request};

librs::main!(main);

const fn round_to_block(num: u64) -> u64 {
    const BLOCK_SIZE: u64 = 512;
    let remainder = num % BLOCK_SIZE;
    if remainder == 0 {
        num
    } else {
        num + (BLOCK_SIZE - remainder)
    }
}

#[repr(transparent)]
#[derive(BinRead)]
struct Octal<const LEN: usize> {
    value: [u8; LEN],
}

impl<const LEN: usize> Octal<LEN> {
    const RADIX: u32 = 8;

    fn as_str(&self) -> &str {
        str::from_utf8(&self.value).unwrap().trim_end_matches('\0')
    }

    fn as_u64(&self) -> u64 {
        u64::from_str_radix(self.as_str(), Self::RADIX).unwrap()
    }
}

impl<const LEN: usize> fmt::Debug for Octal<LEN> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:o}", self.as_u64())
    }
}

#[derive(Debug, PartialEq, Eq)]
#[binrw]
#[brw(repr(u8))]
#[repr(u8)]
enum TypeFlag {
    NormalFile = b'0',
    HardLink = b'1',
    SymbolicLink = b'2',
    CharacterSpecial = b'3',
    BlockSpecial = b'4',
    Directory = b'5',
    Fifo = b'6',
    ContiguousFile = b'7',
    GlobalExtendedHeader = b'g',
    ExtendedHeader = b'x',
}

#[derive(Debug, BinRead)]
#[allow(dead_code)]
pub struct Header {
    #[br(pad_size_to(100))]
    file_name: NullString,

    mode: Octal<8>,
    owner_user_id: Octal<8>,
    group_user_id: Octal<8>,
    size: Octal<12>,
    last_modification_time: Octal<12>,

    // Last two bytes are a null byte and a space, must be ignored
    #[br(pad_size_to(8))]
    checksum: Octal<6>,

    type_flag: TypeFlag,

    #[br(pad_size_to(100))]
    link_name: NullString,

    #[br(magic = b"ustar\0")]
    ustar_version: u16,

    #[br(pad_size_to(32))]
    owner_user_name: NullString,

    #[br(pad_size_to(32))]
    owner_group_name: NullString,

    device_major_number: u64,
    device_minor_number: u64,

    #[br(pad_size_to(32))]
    filename_prefix: NullString,
}

struct File<'a> {
    header: Header,
    content: &'a [u8],
    index: FileIndex,
}

impl fmt::Debug for File<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File")
            .field("name", &self.header.file_name)
            .field("type", &self.header.type_flag)
            .field("contents", &format_args!("[u8; {:#x}]", self.content.len()))
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct TarBall<'a> {
    files: Vec<File<'a>>,
}

impl<'a> TarBall<'a> {
    fn new(file: &'a [u8]) -> Self {
        let mut files = Vec::new();
        let mut cursor = binrw::io::Cursor::new(file);
        let mut index = 0;

        while let Ok(header) = cursor.read_le::<Header>() {
            let content_start = round_to_block(cursor.position());
            let content_end = content_start + header.size.as_u64();
            let content = &cursor.get_ref()[content_start as usize..content_end as usize];

            files.push(File {
                header,
                content,
                index,
            });

            index += 1;
            cursor.set_position(round_to_block(content_end));
        }

        Self { files }
    }

    fn children(&self, parent: FileIndex) -> Option<impl Iterator<Item = &File<'a>>> {
        const ROOT: FileIndex = 0;
        let is_root = parent == ROOT;
        let parent = self.files.get(parent)?;

        if !is_root && parent.header.type_flag != TypeFlag::Directory {
            return None;
        }

        let to_skip = if is_root { 0 } else { 1 };

        Some(
            self.files
                .iter()
                .filter(|f| f.header.file_name.starts_with(&parent.header.file_name))
                .skip(to_skip), // Skip the parent
        )
    }

    fn get_name(&self, name: &[u8]) -> Option<&File<'_>> {
        self.files
            .iter()
            .find(|f| f.header.file_name.as_slice() == name)
    }

    fn get_index(&self, index: FileIndex) -> Option<&File<'_>> {
        self.files.get(index)
    }
}

impl<'a> Index<FileIndex> for TarBall<'a> {
    type Output = File<'a>;

    fn index(&self, index: FileIndex) -> &Self::Output {
        &self.files[index]
    }
}

fn main() {
    librs::syscall::register_server(Some(u64::from_be_bytes(*b"ustar\0\0\0")));

    let size_msg: librs::ipc::Message = virtio::Request::DiskSize.into();
    let size_reply = size_msg.send_receive().unwrap();
    assert_eq!(
        virtio::Reply::from_message(&size_reply),
        Some(virtio::Reply::DiskSize)
    );

    println!("[ustar] reading disk with size {:#x}", size_reply.data[0]);

    let contents_msg: librs::ipc::Message = virtio::Request::ReadDisk.into();
    let contents_reply = contents_msg.send_receive().unwrap();
    let contents = unsafe { virtio::reply_as_slice(&contents_reply).unwrap() };

    let tarball = TarBall::new(contents);

    println!("[ustar] server ready");

    loop {
        let msg = ipc::Message::receive_blocking();
        let reply = ipc::MessageBuilder::new(msg.server_id);

        match Request::from(&msg) {
            Request::ListFiles => {
                let parent = msg.data[0] as usize;

                if let Some(children) = tarball.children(parent).map(|c| c.collect::<Vec<_>>()) {
                    let length = children.chunks(ipc::MessageData::LEN).count() as u64;
                    reply
                        .with_identifier(Reply::ReplyCount.into())
                        .with_data(length.into())
                        .send();

                    for chunk in children.chunks(ipc::MessageData::LEN) {
                        let mut data = [Reply::NoChildren.into(); ipc::MessageData::LEN];
                        for (i, child) in chunk.iter().enumerate() {
                            data[i] = child.index as u64;
                        }

                        reply
                            .with_identifier(Reply::FileIndex.into())
                            .with_data(data.into())
                            .send();
                    }
                } else {
                    reply.with_identifier(Reply::NoChildren.into()).send();
                }
            }

            Request::FileName => {
                let index = msg.data[0] as _;
                if let Some(file) = &tarball.get_index(index) {
                    let name = file.header.file_name.as_slice();
                    reply
                        .with_identifier(Reply::FileName.into())
                        .with_data(name.into())
                        .send();
                } else {
                    reply.with_identifier(Reply::FileNotFound.into()).send();
                }
            }

            Request::FileIndex => {
                let bytes = msg.data.as_be_bytes();
                let bytes = bytes.split(|b| *b == 0).next().unwrap_or(&[]);

                if let Some(fid) = tarball.get_name(bytes).map(|f| f.index as u64) {
                    reply
                        .with_identifier(Reply::FileIndex.into())
                        .with_data(fid.into())
                        .send();
                } else {
                    reply.with_identifier(Reply::FileNotFound.into()).send();
                }
            }

            Request::FileContents => {
                let index = msg.data[0] as _;
                if let Some(file) = &tarball.get_index(index) {
                    if file.header.type_flag == TypeFlag::Directory {
                        reply.with_identifier(Reply::IsDirectory.into()).send();
                    } else {
                        // Allocate a buffer and copy the file contents into it
                        let aligned_size = librs::align_page_up(file.content.len());
                        let mut buffer = alloc::vec![0; aligned_size];
                        buffer[..file.content.len()].copy_from_slice(file.content);
                        let buffer_ptr = buffer.as_mut_ptr() as u64;

                        // Transfer the buffer to the client
                        librs::syscall::transfer_memory(msg.server_id, buffer);
                        let reply_data: &[u64] = &[buffer_ptr, file.content.len() as u64];

                        reply
                            .with_identifier(Reply::FileContents.into())
                            .with_data(reply_data.into())
                            .send();
                    }
                } else {
                    reply.with_identifier(Reply::FileNotFound.into()).send();
                }
            }

            _ => {
                println!("[ustar] unknown request: {:#x}", msg.identifier);
                reply.with_identifier(Reply::UnknownRequest.into()).send();
            }
        }
    }
}
