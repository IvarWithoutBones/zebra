use {
    crate::process::{scheduler, trapframe},
    bitbybit::bitenum,
    core::arch::asm,
};

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

impl SystemCall {
    #[allow(dead_code)]
    #[inline]
    pub fn call(&self) {
        unsafe {
            asm!("ecall", in("a7") self.raw_value());
        }
    }
}

impl TryFrom<u64> for SystemCall {
    type Error = SystemCallError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new_with_raw_value(value).map_err(SystemCallError::Invalid)
    }
}

pub fn handle() {
    let mut procs = scheduler::PROCESSES.lock();
    let proc = procs.current().unwrap();
    let syscall =
        SystemCall::try_from(proc.trap_frame.registers[trapframe::Registers::A7 as usize]);

    if let Ok(syscall) = syscall {
        match syscall {
            SystemCall::Exit => {
                let pid = procs.remove_current().unwrap().pid;
                println!("process {pid} gracefully exited");
                return;
            }

            // `schedule()` will be called by the trap handler
            SystemCall::Yield => {}

            SystemCall::Print => {
                let arg = proc.trap_frame.registers[trapframe::Registers::A0 as usize];
                println!("[user] {}", char::from(arg as u8));
            }

            #[allow(unreachable_patterns)] // Useful when adding new system calls
            _ => unimplemented!("system call {syscall:?}"),
        }

        // Skip past the `ecall` instruction
        proc.trap_frame.registers[trapframe::Registers::ProgramCounter as usize] += 4;
    } else {
        let offender = procs.remove_current().unwrap().pid;
        println!("killed process {offender} because of an invalid system call: {syscall:?}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn raw_value() {
        assert_eq!(SystemCall::Exit.raw_value(), 0);
        assert_eq!(SystemCall::Yield.raw_value(), 1);
        assert_eq!(SystemCall::Print.raw_value(), 2);
    }

    #[test_case]
    fn parse_valid() {
        assert_eq!(SystemCall::Exit, SystemCall::try_from(0).unwrap());
        assert_eq!(SystemCall::Yield, SystemCall::try_from(1).unwrap());
        assert_eq!(SystemCall::Print, SystemCall::try_from(2).unwrap());
    }

    #[test_case]
    fn parse_invalid() {
        assert!(SystemCall::try_from(3).is_err());

        assert_eq!(
            SystemCall::try_from(u64::MAX).unwrap_err(),
            SystemCallError::Invalid(u64::MAX)
        );
    }
}
