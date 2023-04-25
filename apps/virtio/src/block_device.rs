pub const BLOCK_SIZE: usize = 512;

use crate::{
    queue::{DescriptorFlags, Queue, QUEUE_LEN},
    DeviceRegister, DeviceStatus,
};
use alloc::{boxed::Box, fmt};
use bitbybit::bitfield;
use core::{mem::size_of, pin::Pin};
use librs::syscall;

/// Block device: https://docs.oasis-open.org/virtio/virtio/v1.1/csprd01/virtio-v1.1-csprd01.html#x1-2420003
/// Generic: https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-4100006
#[bitfield(u32, default: 0)]
#[derive(PartialEq, Eq)]
pub struct FeatureFlags {
    #[bit(5, rw)]
    read_only: bool,
    #[bit(7, rw)]
    scsi_packet_command: bool,
    #[bit(9, rw)]
    flush_command: bool,
    #[bit(11, rw)]
    config_wce: bool,
    #[bit(12, rw)]
    any_layout: bool,
    #[bit(28, rw)]
    ring_indirect_desc: bool,
    #[bit(29, rw)]
    ring_event_idx: bool,
}

impl From<u32> for FeatureFlags {
    fn from(value: u32) -> Self {
        Self::new_with_raw_value(value)
    }
}

impl fmt::Debug for FeatureFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockDeviceFeatureFlags")
            .field("read_only", &self.read_only())
            .field("scsi_packet_command", &self.scsi_packet_command())
            .field("flush_command", &self.flush_command())
            .field("config_wce", &self.config_wce())
            .field("any_layout", &self.any_layout())
            .field("ring_indirect_desc", &self.ring_indirect_desc())
            .field("ring_event_idx", &self.ring_event_idx())
            .finish()
    }
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-2440004
#[derive(Debug)]
#[repr(C)]
pub struct Geometry {
    cylinders: u16,
    heads: u8,
    sectors: u8,
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-2440004
#[derive(Debug)]
#[repr(C)]
pub struct Topology {
    physical_block_exp: u8,
    alignment_offset: u8,
    minimum_io_size: u16,
    optimal_io_size: u32,
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-2440004
#[repr(C)]
pub struct Config {
    capacity: u64,
    size_max: u32,
    segment_max: u32,
    geometry: Geometry,
    block_size: u32,
    topology: Topology,
    writeback: u8,
    _unused_0: [u8; 3],
    max_discard_sectors: u32,
    max_discard_segments: u32,
    discard_sector_alignment: u32,
    max_write_zeroes_sectors: u32,
    max_write_zeroes_segments: u32,
    write_zeroes_may_unmap: u8,
    _unused_1: [u8; 3],
}

impl Config {
    // TODO: do this in a safe way.
    unsafe fn from_ptr(base: *mut u32) -> Self {
        DeviceRegister::Config
            .into_ptr::<u32, Self>(base)
            .read_volatile()
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockDeviceConfig")
            .field("capacity", &self.capacity)
            .field("size_max", &self.size_max)
            .field("segment_max", &self.segment_max)
            .field("geometry", &self.geometry)
            .field("block_size", &self.block_size)
            .field("topology", &self.topology)
            .field("writeback", &self.writeback)
            .field("max_discard_sectors", &self.max_discard_sectors)
            .field("max_discard_segments", &self.max_discard_segments)
            .field("discard_sector_alignment", &self.discard_sector_alignment)
            .field("max_write_zeroes_sectors", &self.max_write_zeroes_sectors)
            .field("max_write_zeroes_segments", &self.max_write_zeroes_segments)
            .field("write_zeroes_may_unmap", &self.write_zeroes_may_unmap)
            .finish()
    }
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-2500006
#[allow(dead_code)]
#[derive(Debug)]
#[repr(u32)]
pub enum RequestType {
    Read = 0,
    Write = 1,
    Flush = 4,
    Discard = 11,
    WriteZeroes = 13,
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-2500006
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct Request {
    request_type: u32,
    reserved: u32,
    sector: u64,
    data: [u8; 512], // TODO: this is variable
    status: u8,
}

impl Default for Request {
    fn default() -> Self {
        Self {
            request_type: 0,
            reserved: 0,
            sector: 0,
            data: [0; 512],
            status: 0,
        }
    }
}

#[derive(Debug, Copy, Clone)]
struct Info {
    data: Option<usize>,
    status: u8,
    writing: bool,
}

impl Info {
    fn new() -> Self {
        Self {
            status: 0,
            writing: false,
            data: None,
        }
    }
}

#[derive(Debug)]
pub struct BlockDevice {
    queue: Pin<Box<Queue>>,
    device_ptr: *mut u32,
    free: [bool; QUEUE_LEN],
    info: Pin<Box<[Info; QUEUE_LEN]>>, // NOTE: this must be heap allocated as that give us an physical address
    requests: Pin<Box<[Request; QUEUE_LEN]>>,
    used_index: u16,
}

impl BlockDevice {
    // TODO: make some of this generic over the type of device
    /// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-920001
    pub fn init(device_ptr: *mut u32) -> Self {
        // 1. Reset the device.
        DeviceRegister::Status.write(device_ptr, 0);

        // 2. Set the ACKNOWLEDGE status bit: the guest OS has noticed the device.
        let mut status = DeviceStatus::new().with_acknowledge(true);
        DeviceRegister::Status.write(device_ptr, status.raw_value());

        // 3. Set the DRIVER status bit: the guest OS knows how to drive the device.
        status = status.with_driver(true);
        DeviceRegister::Status.write(device_ptr, status.raw_value());

        // 4. Read device feature bits, and write the subset of feature bits understood by the OS and driver to the device.
        //    During this step the driver MAY read (but MUST NOT write) the device-specific configuration fields to check that it can support the device before accepting it.
        let host_features: FeatureFlags = DeviceRegister::DeviceFeatures.read(device_ptr).into();
        let accepted_features = host_features
            .with_read_only(false)
            .with_config_wce(false)
            .with_any_layout(false)
            .with_scsi_packet_command(false)
            .with_ring_indirect_desc(false)
            .with_ring_event_idx(false);
        DeviceRegister::DriverFeatures.write(device_ptr, accepted_features.raw_value());

        // 5. Set the FEATURES_OK status bit. The driver MUST NOT accept new feature bits after this step.
        status = status.with_features_ok(true);
        DeviceRegister::Status.write(device_ptr, status.raw_value());

        // 6. Re-read device status to ensure the FEATURES_OK bit is still set: otherwise, the device does not support our subset of features and the device is unusable.
        status = DeviceStatus::new_with_raw_value(DeviceRegister::Status.read(device_ptr));
        assert!(
            status.features_ok(),
            "device does not support requested features"
        );

        // 7. Perform device-specific setup, including discovery of virtqueues for the device, optional per-bus setup, reading and possibly writing the device’s virtio configuration space, and population of virtqueues.

        // Set the queue selector and size
        DeviceRegister::QueueSelector.write(device_ptr, 0);
        let queue_num_max = DeviceRegister::QueueNumberMax.read(device_ptr);
        assert!(queue_num_max >= QUEUE_LEN as _);
        DeviceRegister::QueueNumber.write(device_ptr, QUEUE_LEN as _);

        // Tell the device about our queue
        let queue = Queue::new();
        let queue_descriptor_ptr = queue.descriptors.as_ptr() as u64;
        DeviceRegister::QueueDescriptorLow.write(device_ptr as *mut _, queue_descriptor_ptr);
        DeviceRegister::QueueDescriptorHigh.write(device_ptr as *mut _, queue_descriptor_ptr >> 32);

        let queue_available_ptr = &queue.available as *const _ as u64;
        DeviceRegister::QueueDriverLow.write(device_ptr as *mut _, queue_available_ptr);
        DeviceRegister::QueueDriverHigh.write(device_ptr as *mut _, queue_available_ptr >> 32);

        let queue_used_ptr = &queue.used as *const _ as u64;
        DeviceRegister::QueueDeviceLow.write(device_ptr as *mut _, queue_used_ptr);
        DeviceRegister::QueueDeviceHigh.write(device_ptr as *mut _, queue_used_ptr >> 32);

        // Enable the queue
        DeviceRegister::QueueReady.write(device_ptr, 1);

        // 8. Set the DRIVER_OK status bit. At this point the device is “live”.
        status = status.with_driver_ok(true);
        DeviceRegister::Status.write(device_ptr, status.raw_value());

        let mut res = BlockDevice {
            queue,
            device_ptr,
            used_index: 0,
            free: [true; QUEUE_LEN],
            info: Box::pin([Info::new(); QUEUE_LEN]),
            requests: Box::pin([Request::default(); QUEUE_LEN]),
        };

        // TODO: remove, this is a hack because we dont clear the BSS
        res.free.iter_mut().for_each(|f| *f = true);

        res
    }

    pub unsafe fn read_sector(&mut self, sector: u64, buffer: *mut u8) {
        let (desc_0, desc_1, desc_2) = {
            (
                self.allocate_descriptor().unwrap(),
                self.allocate_descriptor().unwrap(),
                self.allocate_descriptor().unwrap(),
            )
        };

        let request = &mut self.requests[desc_0];
        request.request_type = RequestType::Read as _;
        request.sector = sector;
        request.reserved = 0;

        self.queue.descriptors[desc_0].addr = request as *mut _ as _;
        self.queue.descriptors[desc_0].len = size_of::<Request>() as _;
        self.queue.descriptors[desc_0].flags = DescriptorFlags::new().with_next(true).raw_value();
        self.queue.descriptors[desc_0].next = desc_1 as _;

        self.queue.descriptors[desc_1].addr = buffer as _;
        self.queue.descriptors[desc_1].len = BLOCK_SIZE as _;
        self.queue.descriptors[desc_1].flags = DescriptorFlags::new()
            .with_write(true)
            .with_next(true)
            .raw_value();
        self.queue.descriptors[desc_1].next = desc_2 as _;

        // The device will write zero  to this upon succesfull completion
        self.info[desc_0].status = u8::MAX;
        let status_ptr = &self.info[desc_0].status as *const _ as _;

        self.queue.descriptors[desc_2].addr = status_ptr;
        self.queue.descriptors[desc_2].len = 1;
        self.queue.descriptors[desc_2].flags = DescriptorFlags::new().with_write(true).raw_value();
        self.queue.descriptors[desc_2].next = 0;

        self.info[desc_0].writing = true;
        self.info[desc_0].data = Some(buffer as _);

        let index = self.queue.available.index as usize % QUEUE_LEN;
        self.queue.available.ring[index] = desc_0 as _;

        librs::memory_sync();
        self.queue.available.index += 1;
        librs::memory_sync();

        // Notify us about queue zero changes
        DeviceRegister::QueueNotify.write(self.device_ptr, 0);

        #[allow(clippy::while_immutable_condition)] // The IRQ handler will change this
        while self.info[desc_0].writing {
            syscall::yield_();
            librs::memory_sync();
        }

        self.free_descriptor_chain(desc_0);
    }

    pub fn interrupt(&mut self) {
        // Acknowledge the interrupt
        let irq_status = DeviceRegister::InterruptStatus.read(self.device_ptr);
        DeviceRegister::InterruptAcknowledge.write(self.device_ptr, irq_status & 0x3);
        librs::memory_sync();

        // Check if the device has finished a request
        while self.used_index != self.queue.used.index {
            let info = {
                let id = self.queue.used.ring[self.used_index as usize % QUEUE_LEN].id as usize;
                &mut self.info[id]
            };

            if info.status != 0 {
                let status = info.status;
                panic!("virtio: request failed with status {status:#x}");
            }

            info.writing = false;
            self.used_index += 1;
            librs::memory_sync();
        }
    }

    fn allocate_descriptor(&mut self) -> Option<usize> {
        self.free.iter().position(|&f| f).map(|index| {
            self.free[index] = false;
            index
        })
    }

    fn free_descriptor(&mut self, index: usize) {
        assert!(index < QUEUE_LEN);
        assert!(!self.free[index]);
        self.queue.descriptors[index].free();
        self.free[index] = true;
    }

    fn free_descriptor_chain(&mut self, mut index: usize) {
        loop {
            let next = self.queue.descriptors[index].next;
            let flags = DescriptorFlags::from(self.queue.descriptors[index].flags);
            self.free_descriptor(index);
            if flags.next() {
                index = next as usize;
            } else {
                break;
            }
        }
    }

    pub fn capacity(&self) -> u64 {
        let config = unsafe { Config::from_ptr(self.device_ptr) };
        config.capacity
    }
}
