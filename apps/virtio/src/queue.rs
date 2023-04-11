use alloc::{boxed::Box, fmt};

use crate::PAGE_SIZE;
use core::mem::size_of;

pub const QUEUE_SIZE: usize = 16;

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-320005
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
struct QueueDescriptor {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-380006
#[derive(Debug)]
#[repr(C)]
struct QueueAvailable {
    flags: u16,
    index: u16,
    ring: [u16; QUEUE_SIZE],
    used_event: u16,
}

impl Default for QueueAvailable {
    fn default() -> Self {
        Self {
            flags: 0,
            index: 0,
            ring: [0; QUEUE_SIZE],
            used_event: 0,
        }
    }
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-430008
#[derive(Debug, Default, Clone, Copy)]
#[repr(C)]
struct QueueUsedElement {
    id: u32,
    len: u32,
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-430008
#[derive(Debug)]
#[repr(C)]
struct QueueUsed {
    flags: u16,
    index: u16,
    ring: [QueueUsedElement; QUEUE_SIZE],
    available_event: u16,
}

impl Default for QueueUsed {
    fn default() -> Self {
        Self {
            flags: 0,
            index: 0,
            ring: [QueueUsedElement { id: 0, len: 0 }; QUEUE_SIZE],
            available_event: 0,
        }
    }
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-260002
#[repr(C)]
pub struct Queue {
    descriptors: [QueueDescriptor; QUEUE_SIZE],
    available: QueueAvailable,
    _padding: [u8; Self::padding_size()],
    used: QueueUsed,
}

impl Queue {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            descriptors: [QueueDescriptor::default(); QUEUE_SIZE],
            available: QueueAvailable::default(),
            _padding: [0; Self::padding_size()],
            used: QueueUsed::default(),
        })
    }

    const fn padding_size() -> usize {
        // TODO: is this correct?
        ((PAGE_SIZE as usize - size_of::<QueueDescriptor>()) * QUEUE_SIZE)
            - size_of::<QueueAvailable>()
    }
}

impl fmt::Debug for Queue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Queue")
            .field("descriptors", &self.descriptors)
            .field("available", &self.available)
            .field("used", &self.used)
            .finish()
    }
}
