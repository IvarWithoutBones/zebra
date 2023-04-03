use core::{arch::asm, mem::size_of, ops::RangeInclusive, time::Duration};

// TODO: merge this with the enum from the kernel
#[derive(Debug, PartialEq, Eq)]
pub enum SystemCall {
    Exit = 0,
    WaitForMessage = 1,
    Sleep = 2,
    Spawn = 3,
    Allocate = 4,
    Deallocate = 5,
    DurationSinceBootup = 6,
    Print = 7,
    Read = 8,
    IdentityMap = 9,
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
pub fn wait_for_message() {
    unsafe {
        asm!("ecall", in("a7") SystemCall::WaitForMessage as usize, options(nomem, nostack));
    }
}

/// Print a string to standard output.
pub fn print(s: &str) {
    unsafe {
        asm!("ecall", in("a7") SystemCall::Print as usize, in("a0") s.as_ptr(), in("a1") s.len(), options(nomem, nostack));
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

/// Send a message to the server with the given ID. The ID is zero-extended to 16 bytes. Panics if the server does not exist.
pub fn send_message<T, B>(server_id: T, message: B)
where
    T: AsRef<[u8]>,
    B: AsRef<[u8]>,
{
    // Zero-extend the name, if necessary.
    let id: [u8; 16] = if server_id.as_ref().len() != 16 {
        let len = server_id.as_ref().len().min(16);
        let mut id = [0u8; 16];
        id[..len].copy_from_slice(server_id.as_ref());
        id
    } else {
        server_id.as_ref().try_into().unwrap()
    };

    let id_msb = u64::from_be_bytes(id.as_ref()[..size_of::<u64>()].try_into().unwrap());
    let id_lsb = u64::from_be_bytes(id.as_ref()[size_of::<u64>()..].try_into().unwrap());

    let msg_ptr = message.as_ref().as_ptr();
    let msg_len = message.as_ref().len();

    unsafe {
        asm!("ecall", in("a7") SystemCall::SendMessage as usize, in("a0") id_msb, in("a1") id_lsb, in("a2") msg_ptr, in("a3") msg_len, options(nomem, nostack));
    }
}

/// Receive a message from a client, returning the client PID and the message or `None` if there is no message available.
pub fn receive_message() -> Option<&'static [u8]> {
    let msg_ptr: *const u8;
    let msg_len: usize;

    unsafe {
        asm!("ecall", in("a7") SystemCall::ReceiveMessage as usize, lateout("a0") msg_ptr, lateout("a1") msg_len, options(nomem, nostack));
    }

    if msg_ptr.is_null() {
        None
    } else {
        Some(unsafe { core::slice::from_raw_parts(msg_ptr, msg_len) })
    }
}

/// Register the current process as a server with the given ID. This may panic if the ID is already in use.
pub fn register_server<T>(server_id: T)
where
    T: AsRef<[u8]>,
{
    // Zero-extend the name, if necessary.
    let id: [u8; 16] = if server_id.as_ref().len() != 16 {
        let len = server_id.as_ref().len().min(16);
        let mut id = [0u8; 16];
        id[..len].copy_from_slice(server_id.as_ref());
        id
    } else {
        server_id.as_ref().try_into().unwrap()
    };

    let id_msb = u64::from_be_bytes(id.as_ref()[..size_of::<u64>()].try_into().unwrap());
    let id_lsb = u64::from_be_bytes(id.as_ref()[size_of::<u64>()..].try_into().unwrap());

    unsafe {
        asm!("ecall", in("a7") SystemCall::RegisterServer as usize, in("a0") id_msb, in("a1") id_lsb, options(nomem, nostack));
    }
}
