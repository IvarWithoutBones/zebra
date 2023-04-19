#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]
#![no_main]

librs::main!(main);

use core::ops::Index;

use alloc::{
    collections::BTreeMap,
    fmt,
    string::{String, ToString},
};
use binrw::{binrw, BinRead, BinReaderExt, NullString};

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
        core::str::from_utf8(&self.value)
            .unwrap()
            .trim_end_matches('\0')
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
}

impl fmt::Debug for File<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("File")
            .field("type", &self.header.type_flag)
            .field("content", &format_args!("[u8; {:#x}]", self.content.len()))
            .finish_non_exhaustive()
    }
}

#[derive(Debug)]
struct TarBall<'a> {
    files: BTreeMap<String, File<'a>>,
}

impl<'a> TarBall<'a> {
    fn new(file: &'a [u8]) -> Self {
        let mut files = BTreeMap::new();
        let mut cursor = binrw::io::Cursor::new(file);

        while let Ok(header) = cursor.read_le::<Header>() {
            let content_start = round_to_block(cursor.position());
            let content_end = content_start + header.size.as_u64();

            let content = &cursor.get_ref()[content_start as usize..content_end as usize];
            files.insert(header.file_name.to_string(), File { header, content });

            cursor.set_position(round_to_block(content_end));
        }

        Self { files }
    }
}

impl<'a> Index<&str> for TarBall<'a> {
    type Output = File<'a>;

    fn index(&self, index: &str) -> &Self::Output {
        self.files.index(index)
    }
}

fn main() {
    librs::syscall::register_server(None);
    println!("[ustar] asking for data");

    let size_msg: librs::ipc::Message = virtio::Request::DiskSize.into();
    let size_reply = size_msg.send_receive().unwrap();
    assert_eq!(
        virtio::Reply::from_message(&size_reply),
        Some(virtio::Reply::DiskSize)
    );
    println!("[ustar] capacity: {:#x}", size_reply.data[0]);

    let contents_msg: librs::ipc::Message = virtio::Request::ReadDisk.into();
    let contents_reply = contents_msg.send_receive().unwrap();
    let contents = unsafe { virtio::reply_as_slice(&contents_reply).unwrap() };

    let tarball = TarBall::new(contents);
    println!("\ndisk contents: {:#?}\n", tarball);

    let str = core::str::from_utf8(tarball["./libs/syscall/Cargo.toml"].content).unwrap();
    println!("```\n{str}```");
}
