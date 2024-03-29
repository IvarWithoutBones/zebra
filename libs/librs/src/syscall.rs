use crate::ipc::MessageData;
use alloc::vec::Vec;
use core::{arch::asm, ops::RangeInclusive, time::Duration};
use syscall::SystemCall;

/// Exit the current process.
pub fn exit() -> ! {
    unsafe {
        asm!("ecall",
            in("a7") SystemCall::Exit as usize,
            options(noreturn, nomem, nostack)
        );
    }
}

/// Give up the CPU until it is scheduled again.
pub fn yield_() {
    unsafe {
        asm!("ecall",
            in("a7") SystemCall::Yield as usize,
            options(nomem, nostack)
        );
    }
}

/// Sleep until an IPC message is received, useful for implementing servers.
pub fn wait_until_message_received() {
    unsafe {
        asm!("ecall",
            in("a7") SystemCall::SleepUntilMessageReceived as usize,
            options(nomem, nostack)
        );
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
            options(nomem, nostack)
        );
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
            options(nomem, nostack)
        );
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
            options(nomem, nostack)
        );
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
            options(nomem, nostack)
        );
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
            options(nomem, nostack)
        );
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
            options(nomem, nostack)
        );
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
            options(nomem, nostack)
        );
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
            options(nomem, nostack)
        );
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
            options(nomem, nostack)
        );
    }

    if server_id == u64::MAX {
        None
    } else {
        Some(server_id)
    }
}

/// Register a function as the handler for a given interrupt, must call `complete_interrupt` when done.
/// Note that this function may not block, nor lock any mutexes. Doing so can cause a deadlock.
pub fn register_interrupt_handler(interrupt: u64, handler: extern "C" fn()) {
    unsafe {
        asm!("ecall",
            in("a0") interrupt,
            in("a1") handler as usize,
            in("a7") SystemCall::RegisterInterruptHandler as usize,
            options(nomem, nostack)
        );
    }
}

/// Complete an interrupt, must always and only be called after an interrupt handler has finished.
pub fn complete_interrupt() -> ! {
    unsafe {
        asm!("ecall",
            in("a7") SystemCall::CompleteInterrupt as usize,
            options(nomem, nostack, noreturn)
        );
    }
}

/// Transfer the given range of memory to the given server. The memory will be unmapped from the current process.
pub fn transfer_memory(to_sid: u64, buffer: Vec<u8>) {
    let aligned_size = super::align_page_up(buffer.len());
    assert!(aligned_size >= buffer.capacity());

    // The process this is mapped into is responsible for deallocation.
    let slice = Vec::leak(buffer);

    let start = slice.as_ptr();
    let end = unsafe { start.add(slice.len()) };

    unsafe {
        asm!("ecall",
            in("a0") to_sid,
            in("a1") start,
            in("a2") end,
            in("a7") SystemCall::TransferMemory as usize,
            options(nomem, nostack)
        );
    }
}
