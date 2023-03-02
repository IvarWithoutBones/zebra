use arbitrary_int::{u10, u3};

pub const BASE_ADDR: usize = 0x0c00_0000;

pub trait InterruptDevice {
    const INTERRUPT_ID: u10;

    fn priority() -> u3;
}

pub fn add_device<T>()
where
    T: InterruptDevice,
{
    set_priority(T::INTERRUPT_ID, T::priority());
    enable_device(T::INTERRUPT_ID);
}

#[allow(clippy::enum_variant_names)] // Just matching the spec
#[repr(usize)]
pub enum Registers {
    SupervisorEnable = 0x2080,
    SupervisorPriority = 0x201000,
    SupervisorClaim = 0x201004,
}

impl Registers {
    unsafe fn into_ptr(self) -> *mut u32 {
        (BASE_ADDR + (self as usize)) as _
    }

    fn read(self) -> u32 {
        unsafe { self.into_ptr().read_volatile() }
    }

    fn write<T>(self, value: T)
    where
        T: Into<u32>,
    {
        unsafe { self.into_ptr().write_volatile(value.into()) }
    }
}

/// Set the source priority for the given interrupt ID
fn set_priority(interrupt_id: u10, priority: u3) {
    unsafe {
        (BASE_ADDR as *mut u32)
            .add(interrupt_id.value().into())
            .write_volatile(priority.into());
    }
}

/// Set the threshold for context 1
pub fn set_global_threshold(threshold: u3) {
    Registers::SupervisorPriority.write(threshold);
}

/// Set the enable bit for the given interrupt ID on context 1
fn enable_device(interrupt_id: u10) {
    let id: u32 = 1 << interrupt_id.value();
    Registers::SupervisorEnable.write(id);
}

/// Claim the next interrupt for context 1
pub fn claim() -> Option<u32> {
    let id = Registers::SupervisorClaim.read();
    if id != 0 {
        Some(id)
    } else {
        None
    }
}

/// Complete an interrupt for context 1. ID should come from `claim()`
pub fn complete(id: u32) {
    Registers::SupervisorClaim.write(id)
}
