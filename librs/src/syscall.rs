use core::{arch::asm, time::Duration};

// TODO: merge this with the enum from the kernel
#[derive(Debug, PartialEq, Eq)]
pub enum SystemCall {
    Exit = 0,
    Yield = 1,
    Sleep = 2,
    Spawn = 3,
    Allocate = 4,
    Deallocate = 5,
    DurationSinceBootup = 6,
    Print = 7,
    Read = 8,
}

/// Exit the current process.
pub fn exit() -> ! {
    unsafe {
        asm!("ecall", in("a7") SystemCall::Exit as usize, options(noreturn, nomem, nostack));
    }
}

/// Yield the current time slice to another process.
pub fn yield_proc() {
    unsafe {
        asm!("ecall", in("a7") SystemCall::Yield as usize, options(nomem, nostack));
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
