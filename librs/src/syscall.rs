use core::arch::asm;

#[derive(Debug, PartialEq, Eq)]
pub enum SystemCall {
    Exit = 0,
    Yield = 1,
    Print = 2,
    Read = 3,
}

/// Exit the current process.
pub fn exit() -> ! {
    unsafe {
        asm!("ecall", in("a7") SystemCall::Exit as usize, options(noreturn));
    }
}

/// Yield the current time slice to another process.
pub fn yield_proc() {
    unsafe {
        asm!("ecall", in("a7") SystemCall::Yield as usize);
    }
}

/// Print a string to standard output.
pub fn print(s: &str) {
    unsafe {
        asm!("ecall", in("a7") SystemCall::Print as usize, in("a0") s.as_ptr(), in("a1") s.len());
    }
}

/// Read a single character from standard input, or `None` if there is no input.
pub fn read() -> Option<char> {
    let result: usize;
    unsafe {
        asm!("ecall", in("a7") SystemCall::Read as usize, out("a0") result);
    }

    if result == 0 {
        None
    } else {
        core::char::from_u32(result as _)
    }
}
