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
const VIRTIO_LEN: u64 = 0x1000;

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

/// Block device: https://docs.oasis-open.org/virtio/virtio/v1.1/csprd01/virtio-v1.1-csprd01.html#x1-2420003
/// Generic: https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-4100006
/// Generic (legacy): https://docs.oasis-open.org/virtio/virtio/v1.1/csprd01/virtio-v1.1-csprd01.html#x1-4130003
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

/// NOTE: This refers to the legacy MMIO interface, as that is what qemu uses.
/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-1560004
#[derive(Debug)]
enum DeviceRegister {
    Magic = 0x0,
    Version = 0x4,
    DeviceId = 0x8,
    VendorId = 0xc,
    HostFeatures = 0x10,
    HostFeaturesSelection = 0x14,
    GuestFeatures = 0x20,
    GuestFeaturesSelection = 0x24,
    GuestPageSize = 0x28,
    QueueSelector = 0x30,
    QueueNumMax = 0x34,
    QueueNum = 0x38,
    QueueAlign = 0x3c,
    QueuePhysicalFrameNumber = 0x40,
    QueueNotify = 0x50,
    InterruptStatus = 0x60,
    InterruptAcknowledge = 0x64,
    Status = 0x70,
    Config = 0x100,
}

impl DeviceRegister {
    #[inline]
    unsafe fn into_ptr<T>(self, base: *mut T) -> *mut T {
        // Looks a bit funky because `byte_offset` isn't stable yet, but whatever
        (base as *mut u8).add(self as _) as *mut T
    }

    #[inline]
    fn read<T>(self, base: *mut T) -> T {
        unsafe { self.into_ptr(base).read_volatile() }
    }

    #[inline]
    fn write<T>(self, base: *mut T, value: T) {
        unsafe { self.into_ptr(base).write_volatile(value) }
    }
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
    fn new(dev_ptr: *mut u32) -> Self {
        // 1. Reset the device.
        DeviceRegister::Status.write(dev_ptr, 0);

        // 2. Set the ACKNOWLEDGE status bit: the guest OS has noticed the device.
        let mut status = DeviceStatus::new().with_acknowledge(true);
        DeviceRegister::Status.write(dev_ptr, status.raw_value());

        // 3. Set the DRIVER status bit: the guest OS knows how to drive the device.
        status = status.with_driver(true);
        DeviceRegister::Status.write(dev_ptr, status.raw_value());

        // 4. Read device feature bits, and write the subset of feature bits understood by the OS and driver to the device.
        //    During this step the driver MAY read (but MUST NOT write) the device-specific configuration fields to check that it can support the device before accepting it.
        let host_features: BlockDeviceFeatureFlags =
            DeviceRegister::HostFeatures.read(dev_ptr).into();
        let accepted_features = host_features
            .with_read_only(false)
            .with_config_wce(false)
            .with_any_layout(false)
            .with_scsi_packet_command(false)
            .with_ring_indirect_desc(false)
            .with_ring_event_idx(false);
        DeviceRegister::GuestFeatures.write(dev_ptr, accepted_features.raw_value());

        // 5. Set the FEATURES_OK status bit. The driver MUST NOT accept new feature bits after this step.
        // NOTE: Omitted in the legacy interface.

        // 6. Re-read device status to ensure the FEATURES_OK bit is still set: otherwise, the device does not support our subset of features and the device is unusable.
        // NOTE: Omitted in the legacy interface.

        // 7. Perform device-specific setup, including discovery of virtqueues for the device, optional per-bus setup, reading and possibly writing the device’s virtio configuration space, and population of virtqueues.
        DeviceRegister::GuestPageSize.write(dev_ptr, PAGE_SIZE as _);

        // Set the queue selector and size
        DeviceRegister::QueueSelector.write(dev_ptr, 0);
        let queue_num_max = DeviceRegister::QueueNumMax.read(dev_ptr);
        assert!(queue_num_max >= QUEUE_SIZE as _);
        DeviceRegister::QueueNum.write(dev_ptr, QUEUE_SIZE as _);

        // Set up a queue and tell the device where it is
        let queue = Queue::new();
        let queue_ptr = Box::into_raw(queue) as usize;
        DeviceRegister::QueuePhysicalFrameNumber
            .write(dev_ptr, queue_ptr as u32 / PAGE_SIZE as u32);

        // 8. Set the DRIVER_OK status bit. At this point the device is “live”.
        status = status.with_driver_ok(true);
        DeviceRegister::Status.write(dev_ptr, status.raw_value());

        BlockDevice {
            queue: unsafe { Box::from_raw(queue_ptr as *mut Queue) },
            device_ptr: dev_ptr,
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
        .step_by(VIRTIO_LEN as _)
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

        if version != 1 {
            println!("unsupported virtio standard {version}");
            continue;
        }

        println!("found {device_id:?} at {dev_ptr:?} ({index}), vendor_id: {vendor_id}");
        match device_id {
            DeviceIdentifier::BlockDevice => virtio.add_block_device(index, dev_ptr),
            _ => println!("unimplemented device type {device_id:?}"),
        }
    }
}
