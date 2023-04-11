use core::{arch::asm, ops::RangeInclusive, time::Duration};
use syscall::SystemCall;

use crate::ipc::MessageData;

/// Exit the current process.
pub fn exit() -> ! {
    unsafe {
        asm!("ecall",
            in("a7") SystemCall::Exit as usize,
            options(noreturn, nomem, nostack));
    }
}

/// Sleep until an IPC message is received, useful for implementing servers.
pub fn wait_until_message_received() {
    unsafe {
        asm!("ecall",
            in("a7") SystemCall::SleepUntilMessageReceived as usize,
            options(nomem, nostack));
    }
}

/// Allocate a block of memory of the given size.
pub fn allocate(size: usize) -> *mut u8 {
    let result: *mut u8;

    unsafe {
        asm!("ecall",
            in("a0") size,
            lateout("a0") result,
            in("a7") SystemCall::Allocate as usize,
            options(nomem, nostack));
    }

    result
}

/// Deallocate a block of memory, may panic if the pointer is invalid.
///
/// # Safety
/// The callee must ensure that the pointer is valid.
pub unsafe fn deallocate(ptr: *mut u8) {
    unsafe {
        asm!("ecall",
            in("a0") ptr,
            in("a7") SystemCall::Deallocate as usize,
            options(nomem, nostack));
    }
}

/// Spawn a new process from an ELF file. If `blocking` is `true`, the current process will block until the new process exits.
pub fn spawn(elf: &[u8], blocking: bool) {
    unsafe {
        asm!("ecall",
            in("a0") elf.as_ptr(),
            in("a1") elf.len(),
            in("a2") blocking as u64,
            in("a7") SystemCall::Spawn as usize,
            options(nomem, nostack));
    }
}

/// The duration since the system was booted.
pub fn duration_since_boot() -> Duration {
    let secs: u64;
    let subsec_nanos: u64;

    unsafe {
        asm!("ecall",
            lateout("a0") secs,
            lateout("a1") subsec_nanos,
            in("a7") SystemCall::DurationSinceBootup as usize,
            options(nomem, nostack));
    }

    Duration::new(secs, subsec_nanos as _)
}

/// Sleep for the given duration.
pub fn sleep(duration: Duration) {
    let secs = duration.as_secs();
    let subsec_nanos = duration.subsec_nanos();

    unsafe {
        asm!("ecall",
            in("a0") secs,
            in("a1") subsec_nanos,
            in("a7") SystemCall::Sleep as usize,
            options(nomem, nostack));
    }
}

/// Identity map the given range of physical memory into the processes address space.
pub fn identity_map(range: RangeInclusive<u64>) {
    let start = *range.start();
    let end = *range.end();

    unsafe {
        asm!("ecall",
            in("a0") start,
            in("a1") end,
            in("a7") SystemCall::IdentityMap as usize,
            options(nomem, nostack));
    }
}

/// Send a message to the server with the given ID.
pub fn send_message(server_id: u64, identifier: u64, data: MessageData) {
    unsafe {
        asm!("ecall",
            in("a0") server_id,
            in("a1") identifier,
            in("a2") data[0],
            in("a3") data[1],
            in("a4") data[2],
            in("a5") data[3],
            in("a6") data[4],
            in("a7") SystemCall::SendMessage as usize,
            options(nomem, nostack));
    }
}

/// Receive a message from a client, returning the identifier and data.
pub fn receive_message() -> Option<(u64, u64, MessageData)> {
    let identifier: u64;
    let sender_sid: u64;
    let mut data = [0; 5];

    unsafe {
        asm!("ecall",
            lateout("a0") identifier,
            lateout("a1") sender_sid,
            lateout("a2") data[0],
            lateout("a3") data[1],
            lateout("a4") data[2],
            lateout("a5") data[3],
            lateout("a6") data[4],
            in("a7") SystemCall::ReceiveMessage as usize,
            options(nomem, nostack));
    }

    if identifier == u64::MAX {
        None
    } else {
        Some((identifier, sender_sid, data.into()))
    }
}

/// Register the current process as a server, optionally with a public name. Returns the server ID.
pub fn register_server(public_name: Option<u64>) -> Option<u64> {
    let public_name = public_name.unwrap_or(0);
    let server_id: u64;

    unsafe {
        asm!("ecall",
            in("a0") public_name,
            lateout("a0") server_id,
            in("a7") SystemCall::RegisterServer as usize,
            options(nomem, nostack));
    }

    if server_id == u64::MAX {
        None
    } else {
        Some(server_id)
    }
}
