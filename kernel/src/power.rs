use core::arch::asm;

// See device-trees/qemu-virt.lds
const SIFIVE_TEST_REG: *mut u8 = 0x100000 as _;

// https://github.com/qemu/qemu/blob/e38d3c5ce5f6e69cf0d87a484d556be0d737d83d/include/hw/misc/sifive_test.h#L39
const NON_ZERO_FLAG: u32 = 0x3333;
const SUCCESS: u32 = 0x5555;
const REBOOT: u32 = 0x7777;
const FAILURE: u32 = to_exit_code(1);

const fn to_exit_code(code: u32) -> u32 {
    // Inverse https://github.com/qemu/qemu/blob/d45a5270d075ea589f0b0ddcf963a5fea1f500ac/hw/misc/sifive_test.c#L39
    (code << 16) | NON_ZERO_FLAG
}

#[allow(dead_code)]
#[repr(u32)]
pub enum ExitType {
    Success,
    Failure,
    Reboot,
    Other(u32),
}

impl From<u32> for ExitType {
    fn from(code: u32) -> Self {
        match code {
            REBOOT => Self::Reboot,
            SUCCESS => Self::Success,
            FAILURE => Self::Failure,
            _ => Self::Other(code),
        }
    }
}

impl Into<u32> for ExitType {
    fn into(self) -> u32 {
        match self {
            Self::Reboot => REBOOT,
            Self::Success => SUCCESS,
            Self::Failure => FAILURE,
            Self::Other(code) => to_exit_code(code),
        }
    }
}

pub fn shutdown(exit_type: ExitType) -> ! {
    let exit_code: u32 = exit_type.into();
    unsafe {
        asm!("sw {code}, 0({ptr})", code = in(reg) exit_code, ptr = in(reg) SIFIVE_TEST_REG);
        loop {
            // We should never reach this if the board is sifive_test compliant, but just in case
            asm!("wfi", options(noreturn, nomem, nostack))
        }
    }
}
