use super::{scheduler, trapframe::Registers};
use crate::{memory, uart};
use bitbybit::bitenum;

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
    Read = 3,
    Allocate = 4,
    Deallocate = 5,
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
    let syscall = SystemCall::try_from(proc.trap_frame.registers[Registers::A7 as usize]);

    if let Ok(syscall) = syscall {
        match syscall {
            SystemCall::Exit => {
                let pid = procs.remove_current().unwrap().pid;
                println!("process {pid} gracefully exited");
                return;
            }

            // `schedule()` will be called by the trap handler immediately afterwards
            SystemCall::Yield => {}

            SystemCall::Print => {
                let str = {
                    let string_ptr = proc.trap_frame.registers[Registers::A0 as usize];
                    let len = proc.trap_frame.registers[Registers::A1 as usize];

                    let ptr = proc.page_table.physical_addr(string_ptr as _).unwrap();
                    // Surely there is no security risk associated with returning arbitrary kernel memory to the end user, with no bounds checking
                    let source = unsafe { core::slice::from_raw_parts(ptr as *const u8, len as _) };
                    core::str::from_utf8(source)
                };

                if let Ok(str) = str {
                    print!("{str}");
                } else {
                    let pid = procs.remove_current().unwrap().pid;
                    println!("invalid string passed from {pid} to SystemCall::Print: {str:?}. Killing process");
                    return;
                }
            }

            SystemCall::Read => {
                let result = uart::UART.lock_with(|uart| uart.poll());
                if let Some(result) = result {
                    proc.trap_frame.registers[Registers::A0 as usize] = result as _;
                } else {
                    // TODO: What would be a better way to indicate that there is no data available?
                    proc.trap_frame.registers[Registers::A0 as usize] = 0;
                }
            }

            SystemCall::Allocate => {
                let size = proc.trap_frame.registers[Registers::A0 as usize] as usize;

                if let Some(result) = memory::allocator().allocate(size) {
                    let allocated_size = memory::align_page_up(size);
                    proc.page_table.identity_map(
                        result as usize,
                        result as usize + allocated_size,
                        memory::page::EntryAttributes::UserReadWrite,
                    );

                    proc.trap_frame.registers[Registers::A0 as usize] = result as _;
                } else {
                    let pid = procs.remove_current().unwrap().pid;
                    println!("failed to allocate memory for process {pid} with size {size:#x}. Killing process");
                    return;
                }
            }

            SystemCall::Deallocate => {
                let ptr = proc.trap_frame.registers[Registers::A0 as usize] as usize;

                // Check if it was mapped in the first place
                if let Some(physical_addr) = proc.page_table.physical_addr(ptr) {
                    let mut alloc = memory::allocator();

                    alloc.deallocate(physical_addr as _);
                    for i in 0..alloc.size_of(physical_addr as _) {
                        proc.page_table.unmap(ptr + i);
                    }
                } else {
                    let pid = procs.remove_current().unwrap().pid;
                    println!("process {pid} attempted to deallocate unmapped memory: {ptr:#x}. Killing process");
                    return;
                }
            }
        }

        // Skip past the `ecall` instruction
        proc.trap_frame.registers[Registers::ProgramCounter as usize] += 4;
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
        assert_eq!(
            SystemCall::try_from(u64::MAX).unwrap_err(),
            SystemCallError::Invalid(u64::MAX)
        );
    }
}
