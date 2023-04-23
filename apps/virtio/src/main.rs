#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]
#![no_main]

//! Resources:
//! https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-1440002
//! https://brennan.io/2020/03/22/sos-block-device

mod block_device;
mod queue;

use crate::block_device::{BlockDevice, BLOCK_SIZE};
use bitbybit::{bitenum, bitfield};
use core::{cell::UnsafeCell, fmt, mem::MaybeUninit, ops::RangeInclusive};
use librs::{ipc, syscall};

librs::main!(main);

// TODO: dont hardcode this
const VIRTIO_RANGE: RangeInclusive<u64> = 0x10001000..=0x10008000;
const VIRTIO_DEVICE_LEN: usize = 0x1000;
const INTERRUPT_ID: u64 = 8;
const MAGIC: u32 = u32::from_le_bytes(*b"virt");

// TODO: lock this up
static mut DISK: UnsafeCell<MaybeUninit<BlockDevice>> = UnsafeCell::new(MaybeUninit::uninit());

/// https://docs.oasis-open.org/virtio/virtio/v1.1/cs01/virtio-v1.1-cs01.html#x1-100001
#[bitfield(u32, default: 0)]
#[derive(PartialEq, Eq)]
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

impl fmt::Debug for DeviceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DeviceStatus")
            .field("acknowledge", &self.acknowledge())
            .field("driver", &self.driver())
            .field("driver_ok", &self.driver_ok())
            .field("features_ok", &self.features_ok())
            .field("device_needs_reset", &self.device_needs_reset())
            .field("failed", &self.failed())
            .finish()
    }
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
#[allow(dead_code)]
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

extern "C" fn interrupt_handler() {
    unsafe { DISK.get_mut().assume_init_mut() }.interrupt();
    syscall::complete_interrupt()
}

fn main() {
    syscall::register_server(Some(123));
    syscall::identity_map(VIRTIO_RANGE);
    syscall::register_interrupt_handler(INTERRUPT_ID, interrupt_handler);

    println!("virtio driver startup");

    for (index, dev_ptr) in VIRTIO_RANGE
        .step_by(VIRTIO_DEVICE_LEN)
        .map(|d| d as *mut u32)
        .enumerate()
    {
        if DeviceRegister::Magic.read(dev_ptr) != MAGIC {
            println!("device {index}: no magic");
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
            DeviceIdentifier::BlockDevice => {
                let block_device = BlockDevice::init(dev_ptr);
                unsafe { DISK.get().write(MaybeUninit::new(block_device)) };
            }
            _ => println!("unimplemented device type {device_id:?}"),
        }
    }

    println!("virtio driver ready");

    loop {
        let msg = ipc::Message::receive_blocking();
        let reply = ipc::MessageBuilder::new(msg.server_id);

        match virtio::Request::from(&msg) {
            virtio::Request::DiskSize => {
                let capacity =
                    unsafe { DISK.get_mut().assume_init_mut() }.capacity() * BLOCK_SIZE as u64;
                reply
                    .with_identifier(virtio::Reply::DiskSize as _)
                    .with_data(capacity.into())
                    .send();
            }

            virtio::Request::ReadDisk => {
                let disk = unsafe { DISK.get_mut().assume_init_mut() };
                let capacity = disk.capacity();

                // Align the buffer to a page so that it can be mapped into the receivers address space
                let size = capacity * BLOCK_SIZE as u64;
                let aligned_size = librs::align_page_up(size as _) as u64;
                let mut buffer = alloc::vec![0u8; aligned_size as usize];

                // Read out every sector of the block device
                for sector in 0..capacity {
                    println!("[virtio] reading sector {sector}");
                    let contents = disk.read_sector(sector);
                    buffer[sector as usize * BLOCK_SIZE..(sector as usize + 1) * BLOCK_SIZE]
                        .copy_from_slice(&contents);
                }

                let buf_ptr = buffer.as_ptr() as u64;
                let reply_data: &[u64] = &[buf_ptr, size];
                println!("[virtio] transferring buffer at {buf_ptr:#x}");

                // Transfer the buffer to the receiver, we cannot access it afterwards
                librs::syscall::transfer_memory(msg.server_id, buffer);

                // Let the receiver know that the data is ready, and it can be found
                reply
                    .with_identifier(virtio::Reply::DataReady as u64)
                    .with_data(reply_data.into())
                    .send();
            }

            virtio::Request::UnknownRequest => {
                reply
                    .with_identifier(virtio::Request::UnknownRequest as u64)
                    .send();
            }
        }
    }
}
