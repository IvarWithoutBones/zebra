use crate::{process, spinlock::SpinLock};

pub const BASE_ADDR: usize = 0x0c00_0000;
const MAX_HANDLERS: usize = 1024;

type InterruptHandler = Option<fn()>;
type UserInterruptHandler = Option<(usize, usize, u32)>;
pub static INTERRUPT_HANDLERS: SpinLock<[(InterruptHandler, UserInterruptHandler); MAX_HANDLERS]> =
    SpinLock::new([(None, None); MAX_HANDLERS]);

pub fn add_user(device_id: u16, pid: usize, handler_ptr: usize) {
    set_priority(device_id, 1);
    enable_device(device_id);

    let handlers = &mut INTERRUPT_HANDLERS.lock();
    assert!(handlers[device_id as usize].0.is_none());
    assert!(handlers[device_id as usize].1.is_none());
    handlers[device_id as usize].1 = Some((pid, handler_ptr, device_id as _));
}

pub fn try_remove_user(pid: usize) -> Option<()> {
    let handlers = &mut INTERRUPT_HANDLERS.lock();
    let device_id = handlers
        .iter()
        .position(|(_, user)| user.map(|(p, _, _)| p == pid).unwrap_or(false))?;
    assert!(handlers[device_id].0.is_none());
    handlers[device_id].1 = None;
    Some(())
}

pub fn handle_interrupt() {
    let mut unlocked_handler = None;

    if let Some(intr) = claim() {
        let lock = &mut INTERRUPT_HANDLERS.lock();
        let (kernel_handler, user_handler) = &mut lock[intr as usize];
        if let Some(handler) = kernel_handler {
            handler();
            complete(intr);
        } else if let Some((pid, handler_ptr, irq_id)) = user_handler {
            // We can only call the user handler *after* releasing the lock
            unlocked_handler = Some((*pid, *handler_ptr, *irq_id));
        } else {
            panic!("no handler for external interrupt with id {intr}");
        }
    }

    if let Some((pid, handler_ptr, irq_id)) = unlocked_handler {
        process::interrupt::handle(irq_id, handler_ptr, pid);
    }
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
fn set_priority(interrupt_id: u16, priority: u8) {
    assert!(priority <= 7);
    assert!(interrupt_id < MAX_HANDLERS as _);
    unsafe {
        (BASE_ADDR as *mut u32)
            .add(interrupt_id.into())
            .write_volatile(priority.into());
    }
}

/// Set the threshold for context 1
pub fn set_global_threshold(threshold: u8) {
    Registers::SupervisorPriority.write(threshold);
}

/// Set the enable bit for the given interrupt ID on context 1
fn enable_device(interrupt_id: u16) {
    let prev_enable = Registers::SupervisorEnable.read();
    let new_enable: u32 = prev_enable | (1 << interrupt_id);
    Registers::SupervisorEnable.write(new_enable);
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
