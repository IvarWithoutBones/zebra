use bitbybit::bitenum;
use core::arch::asm;

#[derive(Debug, PartialEq, Eq)]
pub enum SystemCallError {
    Invalid(u64),
}

#[derive(Debug, PartialEq, Eq)]
#[bitenum(u64)]
pub enum SystemCall {
    Exit = 0,
    Yield = 1,
    Print = 2,
}

impl TryFrom<u64> for SystemCall {
    type Error = SystemCallError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new_with_raw_value(value).map_err(SystemCallError::Invalid)
    }
}

pub fn exit() -> ! {
    unsafe {
        asm!("ecall", in("a7") SystemCall::Exit as usize, options(noreturn));
    }
}

pub fn print(s: &str) {
    unsafe {
        asm!("ecall", in("a7") SystemCall::Print as usize, in("a0") s.as_ptr(), in("a1") s.len());
    }
}
