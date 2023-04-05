use core::{arch::asm, ops::RangeInclusive, time::Duration};

// TODO: merge this with the enum from the kernel
#[derive(Debug, PartialEq, Eq)]
pub enum SystemCall {
    Exit = 0,
    Sleep = 2,
    Spawn = 3,
    Allocate = 4,
    Deallocate = 5,
    DurationSinceBootup = 6,
    Read = 8,
    IdentityMap = 9,
    WaitUntilMessageReceived = 1,
    SendMessage = 10,
    ReceiveMessage = 11,
    RegisterServer = 12,
}

/// Exit the current process.
pub fn exit() -> ! {
    unsafe {
        asm!("ecall", in("a7") SystemCall::Exit as usize, options(noreturn, nomem, nostack));
    }
}

/// Sleep until an IPC message is received, useful for implementing servers.
pub fn wait_until_message_received() {
    unsafe {
        asm!("ecall", in("a7") SystemCall::WaitUntilMessageReceived as usize, options(nomem, nostack));
    }
}

/// Read a single character from standard input, or `None` if there is no input.
pub fn read() -> Option<char> {
    let result: usize;

    unsafe {
        asm!("ecall", in("a7") SystemCall::Read as usize, out("a0") result, options(nomem, nostack));
    }

    if result == 0 {
        None
    } else {
        core::char::from_u32(result as _)
    }
}

/// Allocate a block of memory of the given size.
pub fn allocate(size: usize) -> *mut u8 {
    let result: *mut u8;

    unsafe {
        asm!("ecall", in("a7") SystemCall::Allocate as usize, in("a0") size, lateout("a0") result, options(nomem, nostack));
    }

    result
}

/// Deallocate a block of memory, may panic if the pointer is invalid.
///
/// # Safety
/// The callee must ensure that the pointer is valid.
pub unsafe fn deallocate(ptr: *mut u8) {
    unsafe {
        asm!("ecall", in("a7") SystemCall::Deallocate as usize, in("a0") ptr, options(nomem, nostack));
    }
}

/// Spawn a new process from an ELF file. If `blocking` is `true`, the current process will block until the new process exits.
pub fn spawn(elf: &[u8], blocking: bool) {
    unsafe {
        asm!("ecall", in("a7") SystemCall::Spawn as usize, in("a0") elf.as_ptr(), in("a1") elf.len(), in("a2") blocking as u64, options(nomem, nostack));
    }
}

/// The duration since the system was booted.
pub fn duration_since_boot() -> Duration {
    let secs: u64;
    let subsec_nanos: u64;

    unsafe {
        asm!("ecall", in("a7") SystemCall::DurationSinceBootup as usize, lateout("a0") secs, lateout("a1") subsec_nanos, options(nomem, nostack));
    }

    Duration::new(secs, subsec_nanos as _)
}

/// Sleep for the given duration.
pub fn sleep(duration: Duration) {
    let secs = duration.as_secs();
    let subsec_nanos = duration.subsec_nanos();
    unsafe {
        asm!("ecall", in("a7") SystemCall::Sleep as usize, in("a0") secs, in("a1") subsec_nanos, options(nomem, nostack));
    }
}

/// Identity map the given range of physical memory into the processes address space.
pub fn identity_map(range: RangeInclusive<u64>) {
    let start = *range.start();
    let end = *range.end();

    unsafe {
        asm!("ecall", in("a7") SystemCall::IdentityMap as usize, in("a0") start, in("a1") end, options(nomem, nostack));
    }
}

/// Send a message to the server with the given ID.
pub fn send_message(server_id: u64, identifier: u64, data: u64) {
    unsafe {
        asm!("ecall", in("a7") SystemCall::SendMessage as usize, in("a0") server_id, in("a1") identifier, in("a2") data, options(nomem, nostack));
    }
}

/// Receive a message from a client, returning the identifier and data.
pub fn receive_message() -> Option<(u64, u64)> {
    let identifier: u64;
    let data: u64;

    unsafe {
        asm!("ecall", in("a7") SystemCall::ReceiveMessage as usize, lateout("a0") identifier, lateout("a1") data, options(nomem, nostack));
    }

    if identifier == u64::MAX {
        None
    } else {
        Some((identifier, data))
    }
}

/// Register the current process as a server, optionally with a public name. Returns the server ID.
pub fn register_server(public_name: Option<u64>) -> Option<u64> {
    let public_name = public_name.unwrap_or(0);
    let server_id: u64;

    unsafe {
        asm!("ecall", in("a7") SystemCall::RegisterServer as usize, in("a0") public_name, lateout("a0") server_id, options(nomem, nostack));
    }

    if server_id == u64::MAX {
        None
    } else {
        Some(server_id)
    }
}
