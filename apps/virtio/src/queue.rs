use alloc::boxed::Box;
use bitbybit::bitfield;
use core::pin::Pin;

pub const QUEUE_LEN: usize = 16;

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-320005
#[derive(Debug, Default, Clone, Copy)]
#[repr(C, align(16))]
pub struct Descriptor {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

impl Descriptor {
    pub fn free(&mut self) {
        self.addr = 0;
        self.len = 0;
        self.flags = 0;
        self.next = 0;
    }
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-320005
#[bitfield(u16, default: 0)]
#[derive(Debug)]
pub struct DescriptorFlags {
    #[bit(0, rw)]
    pub next: bool,
    #[bit(1, rw)]
    pub write: bool,
    #[bit(2, rw)]
    pub indirect: bool,
}

impl From<u16> for DescriptorFlags {
    fn from(flags: u16) -> Self {
        Self::new_with_raw_value(flags)
    }
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-380006
#[derive(Debug)]
#[repr(C, align(2))]
pub struct Available {
    pub flags: u16,
    pub index: u16,
    pub ring: [u16; QUEUE_LEN],
    pub used_event: u16,
}

impl Default for Available {
    fn default() -> Self {
        Self {
            flags: 0,
            index: 0,
            ring: [0; QUEUE_LEN],
            used_event: 0,
        }
    }
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-430008
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
pub struct UsedElement {
    pub id: u32,
    pub len: u32,
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-430008
#[derive(Debug, Default)]
#[repr(C, align(4))]
pub struct Used {
    pub flags: u16,
    pub index: u16,
    pub ring: [UsedElement; QUEUE_LEN],
    pub available_event: u16,
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-260002
#[derive(Debug)]
#[repr(C, align(4096))]
pub struct Queue {
    pub descriptors: [Descriptor; QUEUE_LEN],
    pub available: Available,
    pub used: Used,
}

impl Queue {
    pub fn new() -> Pin<Box<Self>> {
        Box::pin(Self {
            descriptors: [Descriptor::default(); QUEUE_LEN],
            available: Available::default(),
            used: Used::default(),
        })
    }
}
