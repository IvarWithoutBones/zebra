use super::{Process, ProcessState};
use crate::{ipc, spinlock::SpinLock, trap::clint};
use alloc::collections::VecDeque;
use core::arch::asm;

pub static PROCESSES: SpinLock<ProcessList> = SpinLock::new(ProcessList::new());

pub struct ProcessList {
    pub processes: VecDeque<Process>,
}

impl ProcessList {
    const fn new() -> Self {
        Self {
            processes: VecDeque::new(),
        }
    }

    pub fn push(&mut self, process: Process) {
        self.processes.push_back(process);
    }

    pub fn remove_current(&mut self) -> Option<Process> {
        let proc = self.processes.pop_front()?;

        // Let the parent process continue if it was waiting on us
        self.processes
            .iter_mut()
            .find(|p| match p.state {
                ProcessState::ChildExited { child_pid } => child_pid == proc.pid,
                _ => false,
            })
            .map(|p| p.state = ProcessState::Ready)
            .unwrap_or(());

        Some(proc)
    }

    pub fn next(&mut self) -> Option<&mut Process> {
        self.processes.rotate_right(1);
        self.processes.front_mut()
    }

    pub fn current(&mut self) -> Option<&mut Process> {
        self.processes.front_mut()
    }

    pub fn find_by_pid(&mut self, pid: usize) -> Option<&mut Process> {
        self.processes.iter_mut().find(|p| p.pid == pid)
    }
}

pub fn insert(process: Process) {
    PROCESSES.lock_with(|processes| processes.push(process));
}

pub fn schedule() -> ! {
    loop {
        let proc: Option<&mut Process> = PROCESSES.lock_with(|procs| {
            if let Some(current) = procs.current() {
                if current.state == ProcessState::Running {
                    current.state = ProcessState::Ready;
                }
            }

            let len = procs.processes.len();
            let mut next_proc = procs.next().expect("no processes to schedule");
            let mut found = false;

            for _ in 0..len {
                match next_proc.state {
                    ProcessState::Ready | ProcessState::HandlingInterrupt { .. } => {
                        found = true;
                        break;
                    }

                    ProcessState::Sleeping { duration } => {
                        if clint::time_since_bootup() >= duration {
                            next_proc.state = ProcessState::Ready;
                            found = true;
                            break;
                        }
                    }

                    ProcessState::MessageSent { receiver_sid } => {
                        let mut server_list = ipc::server_list().lock();
                        let server = server_list.get_by_sid(receiver_sid).unwrap_or_else(|| {
                            panic!(
                                "attempted to look up non-existent server with SID {receiver_sid}!"
                            )
                        });

                        if server.has_messages() {
                            if let Some(server_proc) = procs.find_by_pid(server.process_id) {
                                if let ProcessState::WaitUntilMessageReceived = server_proc.state {
                                    server_proc.state = ProcessState::Ready;
                                }
                            } else {
                                panic!(
                                    "attempted to wake up non-existent process with PID {}!",
                                    server.process_id
                                );
                            }
                        }
                    }

                    _ => (),
                }

                next_proc = procs.next().expect("no processes to schedule");
            }

            if !found {
                return None;
            }

            if let ProcessState::HandlingInterrupt { .. } = next_proc.state {
                // Do nothing
            } else {
                next_proc.state = ProcessState::Running;
            }

            // We need a reference to the process that remains valid *after* dropping the PROCESSES lock,
            // should probably use a smart pointer instead of the unsafe raw pointer.
            let next_proc = next_proc as *mut _ as *mut Process;
            Some(unsafe { &mut *next_proc })
        });

        if let Some(proc) = proc {
            proc.run()
        } else {
            // We should never get here unless all processes are non-runnable, in which case we wait for an interrupt to wake us up to avoid a busy loop.
            unsafe { asm!("wfi") }
        }
    }
}
