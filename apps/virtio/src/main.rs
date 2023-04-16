#![allow(dead_code)] // TODO: remove
#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]
#![no_main]

//! https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-1440002

mod queue;

use crate::queue::{Queue, QUEUE_SIZE};
use alloc::boxed::Box;
use bitbybit::{bitenum, bitfield};
use core::{fmt, ops::RangeInclusive};
use librs::syscall;

librs::main!(main);

const PAGE_SIZE: u64 = 4096;

// TODO: dont hardcode this
const VIRTIO_RANGE: RangeInclusive<u64> = 0x10001000..=0x10008000;
const VIRTIO_LEN: usize = 0x1000;

const MAGIC: u32 = u32::from_le_bytes(*b"virt");

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-100001
#[bitfield(u32, default: 0)]
#[derive(Debug, PartialEq, Eq)]
struct DeviceStatus {
    #[bit(0, rw)]
    acknowledge: bool,
    #[bit(1, rw)]
    driver: bool,
    #[bit(2, rw)]
    driver_ok: bool,
    #[bit(3, rw)]
    features_ok: bool,
    #[bit(6, rw)]
    device_needs_reset: bool,
    #[bit(7, rw)]
    failed: bool,
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-1930005
#[bitenum(u32, exhaustive: false)]
#[derive(Debug, PartialEq, Eq)]
enum DeviceIdentifier {
    Invalid = 0,
    NetworkCard = 1,
    BlockDevice = 2,
    Console = 3,
    EntropySource = 4,
    MemoryBallooning = 5,
    IoMemory = 6,
    Rpmsg = 7,
    SCSIHost = 8,
    NinePTransport = 9,
    Mac80211Wlan = 10,
    RprocSerial = 11,
    VirtioCAIF = 12,
    MemoryBalloon = 13,
    GPUDevice = 16,
    TimerClockDevice = 17,
    InputDevice = 18,
    SocketDevice = 19,
    CryptoDevice = 20,
    SignalDistributionModule = 21,
    PstoreDevice = 22,
    IOMMUDevice = 23,
    MemoryDevice = 24,
}

impl TryFrom<u32> for DeviceIdentifier {
    type Error = u32;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        Self::new_with_raw_value(value)
    }
}

/// NOTE: this refers to the non-legacy version of MMIO registers.
/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-1460002
#[derive(Debug)]
enum DeviceRegister {
    Magic = 0x0,
    Version = 0x4,
    DeviceId = 0x8,
    VendorId = 0xc,
    DeviceFeatures = 0x10,
    DeviceFeatureSelection = 0x14,
    DriverFeatures = 0x20,
    DriverFeatureSelection = 0x24,
    QueueSelector = 0x30,
    QueueNumberMax = 0x34,
    QueueNumber = 0x38,
    QueueReady = 0x44,
    QueueNotify = 0x50,
    InterruptStatus = 0x60,
    InterruptAcknowledge = 0x64,
    Status = 0x70,
    QueueDescriptorLow = 0x80,
    QueueDescriptorHigh = 0x84,
    QueueDriverLow = 0x90,
    QueueDriverHigh = 0x94,
    QueueDeviceLow = 0xa0,
    QueueDeviceHigh = 0xa4,
    ConfigGeneration = 0xfc,
    Config = 0x100, // NOTE: this is an offset, not a register
}

impl DeviceRegister {
    #[inline]
    unsafe fn into_ptr<T, R>(self, base: *mut T) -> *mut R {
        // Looks a bit funky because `byte_offset` isn't stable yet, but whatever
        (base as *mut u8).add(self as _) as *mut R
    }

    #[inline]
    fn read<T>(self, base: *mut T) -> T {
        unsafe { self.into_ptr::<T, T>(base).read_volatile() }
    }

    #[inline]
    fn write<T>(self, base: *mut T, value: T) {
        unsafe { self.into_ptr::<T, T>(base).write_volatile(value) }
    }
}

/// Block device: https://docs.oasis-open.org/virtio/virtio/v1.1/csprd01/virtio-v1.1-csprd01.html#x1-2420003
/// Generic: https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-4100006
#[bitfield(u32, default: 0)]
#[derive(PartialEq, Eq)]
struct BlockDeviceFeatureFlags {
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

impl From<u32> for BlockDeviceFeatureFlags {
    fn from(value: u32) -> Self {
        Self::new_with_raw_value(value)
    }
}

impl fmt::Debug for BlockDeviceFeatureFlags {
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
struct BlockDeviceGeometry {
    cylinders: u16,
    heads: u8,
    sectors: u8,
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-2440004
#[derive(Debug)]
#[repr(C)]
struct BlockDeviceTopology {
    physical_block_exp: u8,
    alignment_offset: u8,
    minimum_io_size: u16,
    optimal_io_size: u32,
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-2440004
#[repr(C)]
struct BlockDeviceConfig {
    capacity: u64,
    size_max: u32,
    segment_max: u32,
    geometry: BlockDeviceGeometry,
    topology: BlockDeviceTopology,
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

impl BlockDeviceConfig {
    // TODO: do this in a safe way.
    unsafe fn from_ptr(base: *mut u32) -> Self {
        DeviceRegister::Config
            .into_ptr::<u32, Self>(base)
            .read_volatile()
    }
}

impl fmt::Debug for BlockDeviceConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockDeviceConfig")
            .field("capacity", &self.capacity)
            .field("size_max", &self.size_max)
            .field("segment_max", &self.segment_max)
            .field("geometry", &self.geometry)
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
#[derive(Debug)]
#[repr(u32)]
enum BlockDeviceRequestType {
    Read = 0,
    Write = 1,
    Flush = 4,
    Discard = 11,
    WriteZeroes = 13,
}

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-2500006
#[derive(Debug)]
#[repr(C)]
struct BlockDeviceRequest {
    request_type: u32,
    reserved: u32,
    sector: u64,
    data: [u8; 512], // TODO: this is variable
    status: u8,
}

#[derive(Debug)]
struct BlockDevice {
    queue: Box<Queue>,
    device_ptr: *mut u32,
    index: usize,
    acknowledge_used_index: u16,
    read_only: bool,
}

impl BlockDevice {
    /// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-920001
    fn new(device_ptr: *mut u32) -> Self {
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
        let host_features: BlockDeviceFeatureFlags =
            DeviceRegister::DeviceFeatures.read(device_ptr).into();
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
        let config = unsafe { BlockDeviceConfig::from_ptr(device_ptr) };
        println!("{config:#?}");

        // Set the queue selector and size
        DeviceRegister::QueueSelector.write(device_ptr, 0);
        let queue_num_max = DeviceRegister::QueueNumberMax.read(device_ptr);
        assert!(queue_num_max >= QUEUE_SIZE as _);
        DeviceRegister::QueueNumber.write(device_ptr, QUEUE_SIZE as _);

        // TODO: tell the device about the queue
        let queue = Queue::new();

        // 8. Set the DRIVER_OK status bit. At this point the device is “live”.
        status = status.with_driver_ok(true);
        DeviceRegister::Status.write(device_ptr, status.raw_value());

        BlockDevice {
            queue,
            device_ptr,
            index: 0,
            acknowledge_used_index: 0,
            read_only: true,
        }
    }
}

#[derive(Debug)]
struct VirtIO {
    block_devices: [Option<BlockDevice>; 8],
}

impl VirtIO {
    const fn new() -> Self {
        Self {
            block_devices: [None, None, None, None, None, None, None, None],
        }
    }

    fn add_block_device(&mut self, index: usize, dev_ptr: *mut u32) {
        self.block_devices[index] = Some(BlockDevice::new(dev_ptr));
    }
}

fn main() {
    syscall::register_server(None);
    syscall::identity_map(VIRTIO_RANGE);

    let mut virtio = VirtIO::new();

    for dev in virtio.block_devices.iter() {
        assert!(dev.is_none());
    }

    println!("virtio driver startup");

    for (index, dev_ptr) in VIRTIO_RANGE
        .step_by(VIRTIO_LEN)
        .map(|d| d as *mut u32)
        .enumerate()
    {
        if DeviceRegister::Magic.read(dev_ptr) != MAGIC {
            continue;
        }

        let device_id: DeviceIdentifier =
            DeviceRegister::DeviceId.read(dev_ptr).try_into().unwrap();
        let vendor_id = DeviceRegister::VendorId.read(dev_ptr);
        let version = DeviceRegister::Version.read(dev_ptr);

        if device_id == DeviceIdentifier::Invalid {
            continue;
        }

        match version {
            1 => {
                println!("device {index}: unsupported legacy virtio standard {version}");
                continue;
            }

            2 => (),

            _ => {
                println!("device {index}: unsupported virtio standard {version}");
                continue;
            }
        }

        println!("found {device_id:?} {index} at {dev_ptr:?}: vendor_id = {vendor_id}, virtio = {version}");
        match device_id {
            DeviceIdentifier::BlockDevice => virtio.add_block_device(index, dev_ptr),
            _ => println!("unimplemented device type {device_id:?}"),
        }
    }
}
