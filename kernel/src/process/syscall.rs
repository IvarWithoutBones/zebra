use super::{scheduler, trapframe::Registers, Process, ProcessState};
use crate::{
    ipc::{self, Message},
    memory,
    trap::clint,
};
use bitbybit::bitenum;
use core::time::Duration;

#[derive(Debug, PartialEq, Eq)]
pub enum SystemCallError {
    Invalid(u64),
}

#[derive(Debug, PartialEq, Eq)]
#[bitenum(u64)]
pub enum SystemCall {
    Exit = 0,
    WaitUntilMessageReceived = 1,
    Sleep = 2,
    Spawn = 3,
    Allocate = 4,
    Deallocate = 5,
    DurationSinceBootup = 6,
    IdentityMap = 9,
    SendMessage = 10,
    ReceiveMessage = 11,
    RegisterServer = 12,
}

impl TryFrom<u64> for SystemCall {
    type Error = SystemCallError;

    fn try_from(value: u64) -> Result<Self, Self::Error> {
        Self::new_with_raw_value(value).map_err(SystemCallError::Invalid)
    }
}

pub fn handle() -> Option<()> {
    let mut procs = scheduler::PROCESSES.lock();
    let proc = procs.current().unwrap();
    let syscall = SystemCall::try_from(proc.trap_frame.registers[Registers::A7 as usize]);

    // Skip past the `ecall` instruction
    proc.trap_frame.registers[Registers::ProgramCounter as usize] += 4;

    if let Ok(syscall) = syscall {
        match syscall {
            SystemCall::Exit => {
                let pid = procs.remove_current().unwrap().pid;
                println!("process {pid} gracefully exited");
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
                }
            }

            SystemCall::Deallocate => {
                let ptr = proc.trap_frame.registers[Registers::A0 as usize] as usize;

                // Check if it was mapped in the first place
                if let Some(physical_addr) = proc.page_table.physical_addr(ptr) {
                    let mut alloc = memory::allocator();
                    for i in 0..alloc.size_of(physical_addr as _) {
                        proc.page_table.unmap(ptr + i);
                    }
                    alloc.deallocate(physical_addr as _);
                } else {
                    let pid = procs.remove_current().unwrap().pid;
                    println!("process {pid} attempted to deallocate unmapped memory: {ptr:#x}. Killing process");
                }
            }

            SystemCall::Spawn => {
                let elf_ptr = proc.trap_frame.registers[Registers::A0 as usize];
                let elf_size = proc.trap_frame.registers[Registers::A1 as usize];
                let blocking = proc.trap_frame.registers[Registers::A2 as usize] != 0;

                let elf = {
                    let elf_ptr = { proc.page_table.physical_addr(elf_ptr as _)? };
                    // The safety concerns from SystemCall::Print apply here as well
                    unsafe { core::slice::from_raw_parts(elf_ptr as *const u8, elf_size as _) }
                };

                let new_proc = if blocking {
                    let new_proc = Process::new(elf);
                    proc.state = ProcessState::ChildExited {
                        child_pid: new_proc.pid,
                    };
                    new_proc
                } else {
                    Process::new(elf)
                };

                procs.push(new_proc);
            }

            SystemCall::DurationSinceBootup => {
                let time = clint::time_since_bootup();
                proc.trap_frame.registers[Registers::A0 as usize] = time.as_secs() as _;
                proc.trap_frame.registers[Registers::A1 as usize] = time.subsec_nanos() as _;
            }

            SystemCall::Sleep => {
                let duration = {
                    let seconds = proc.trap_frame.registers[Registers::A0 as usize];
                    let nanoseconds = proc.trap_frame.registers[Registers::A1 as usize] as u32;
                    Duration::new(seconds, nanoseconds)
                };

                let duration = clint::time_since_bootup() + duration;
                proc.state = ProcessState::Sleeping { duration };
            }

            // TODO: Capabilities, not every process should be allowed to do this.
            // TODO: Maybe it would make more sense to only allow this when spawing a new process?
            SystemCall::IdentityMap => {
                let start = proc.trap_frame.registers[Registers::A0 as usize] as usize;
                let end = proc.trap_frame.registers[Registers::A1 as usize] as usize;

                let root_table = memory::page::root_table();
                // TODO: this will not work if the given address is not already mapped by the kernel.
                let physical_start = root_table.physical_addr(start).unwrap();
                let physical_end = root_table.physical_addr(end).unwrap();

                proc.page_table.identity_map(
                    physical_start,
                    physical_end,
                    memory::page::EntryAttributes::UserReadWrite, // Execute permissions dont seem like a good idea
                );
            }

            SystemCall::WaitUntilMessageReceived => {
                proc.state = ProcessState::MessageReceived;
            }

            SystemCall::SendMessage => {
                let server_id = proc.trap_frame.registers[Registers::A0 as usize];
                let identifier = proc.trap_frame.registers[Registers::A1 as usize];
                let data = proc.trap_frame.registers[Registers::A2 as usize];

                let mut server_list = ipc::server_list().lock();
                let curr_sid = if let Some(server) = server_list.get_by_pid(proc.pid) {
                    server.server_id
                } else {
                    let pid = procs.remove_current().unwrap().pid;
                    println!("process {pid} tried to send a message without being a server. Killing process");
                    return None;
                };

                if let Some(server) = server_list.get_by_sid(server_id) {
                    server.send_message(Message::new(proc.pid, curr_sid, identifier, data));

                    proc.state = ProcessState::MessageSent {
                        receiver_sid: server.server_id,
                    };

                    procs.find_by_pid(server.process_id)?.state = ProcessState::Ready;
                } else {
                    let pid = procs.remove_current().unwrap().pid;
                    println!("process {pid} tried to send a message to a non-existent server {server_id}. Killing process");
                }
            }

            SystemCall::ReceiveMessage => {
                let mut server_list = ipc::server_list().lock();
                let server = server_list.get_by_pid(proc.pid).unwrap();

                if let Some(msg) = server.receive_message() {
                    proc.trap_frame.registers[Registers::A0 as usize] = msg.identifier;
                    proc.trap_frame.registers[Registers::A1 as usize] = msg.data;
                    proc.trap_frame.registers[Registers::A2 as usize] = msg.sender_sid;

                    let mut sender = procs.find_by_pid(msg.sender_pid)?;
                    if let ProcessState::MessageSent { receiver_sid } = sender.state {
                        if receiver_sid == server.server_id {
                            sender.state = ProcessState::Ready;
                        }
                    }
                } else {
                    proc.trap_frame.registers[Registers::A0 as usize] = u64::MAX;
                }
            }

            SystemCall::RegisterServer => {
                let public_name = proc.trap_frame.registers[Registers::A0 as usize];

                let server_id = if public_name != 0 {
                    ipc::server_list()
                        .lock()
                        .register(proc.pid, Some(public_name))
                } else {
                    ipc::server_list().lock().register(proc.pid, None)
                };

                proc.trap_frame.registers[Registers::A0 as usize] = server_id.unwrap_or(u64::MAX);
            }
        }
    } else {
        let offender = procs.remove_current().unwrap().pid;
        println!("killed process {offender} because of an invalid system call: {syscall:?}");
    }

    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test_case]
    fn raw_value() {
        assert_eq!(SystemCall::Exit.raw_value(), 0);
        assert_eq!(SystemCall::WaitUntilMessageReceived.raw_value(), 1);
        assert_eq!(SystemCall::Sleep.raw_value(), 2);
    }

    #[test_case]
    fn parse_valid() {
        assert_eq!(SystemCall::Exit, SystemCall::try_from(0).unwrap());
        assert_eq!(
            SystemCall::WaitUntilMessageReceived,
            SystemCall::try_from(1).unwrap()
        );
        assert_eq!(SystemCall::Sleep, SystemCall::try_from(2).unwrap());
    }

    #[test_case]
    fn parse_invalid() {
        assert_eq!(
            SystemCall::try_from(u64::MAX).unwrap_err(),
            SystemCallError::Invalid(u64::MAX)
        );
    }
}
