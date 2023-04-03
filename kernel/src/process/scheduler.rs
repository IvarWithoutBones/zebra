use super::{Process, ProcessState, WaitCondition};
use crate::{spinlock::Spinlock, trap::clint};
use alloc::vec::Vec;
use core::arch::asm;

pub static PROCESSES: Spinlock<ProcessList> = Spinlock::new(ProcessList::new());

pub struct ProcessList {
    processes: Vec<Process>,
    current: Option<usize>,
}

impl ProcessList {
    const fn new() -> Self {
        Self {
            processes: Vec::new(),
            current: None,
        }
    }

    pub fn push(&mut self, process: Process) {
        self.processes.push(process);
    }

    pub fn remove_current(&mut self) -> Option<Process> {
        let proc = self.processes.remove(self.current?);

        // Let the parent process continue if it was waiting on us
        self.processes
            .iter_mut()
            .find(|p| match p.state {
                ProcessState::Waiting(WaitCondition::ChildExit { child_pid }) => {
                    child_pid == proc.pid
                }
                _ => false,
            })
            .map(|p| p.state = ProcessState::Ready)
            .unwrap_or(());

        Some(proc)
    }

    pub fn next(&mut self) -> Option<&Process> {
        let next = self.current.map_or(0, |curr| {
            curr.wrapping_add(1)
                .checked_rem(self.processes.len())
                .unwrap_or(0)
        });

        let result = self.processes.get(next)?;
        self.current = Some(next);
        Some(result)
    }

    pub fn current(&mut self) -> Option<&mut Process> {
        self.processes.get_mut(self.current?)
    }

    pub fn find_by_pid(&mut self, pid: usize) -> Option<&mut Process> {
        self.processes.iter_mut().find(|p| p.pid == pid)
    }

    pub fn update_waiting(&mut self) {
        let now = clint::time_since_bootup();
        for proc in self.processes.iter_mut() {
            if let ProcessState::Waiting(WaitCondition::Duration(duration)) = proc.state {
                if now >= duration {
                    proc.state = ProcessState::Ready;
                }
            }
        }
    }
}

pub fn insert(process: Process) {
    PROCESSES.lock_with(|processes| processes.push(process));
}

pub fn schedule() -> ! {
    // We need a reference to the process that remains valid *after* dropping the PROCESSES lock,
    // should probably use a smart pointer instead of the unsafe raw pointer.
    PROCESSES
        .lock_with(|procs| {
            if let Some(current) = procs.current() {
                if current.state == ProcessState::Running {
                    current.state = ProcessState::Ready;
                }
            }

            let mut next_proc: *mut Process =
                procs.next().expect("no processes to schedule") as *const _ as _;

            // Find a process that is waiting to run
            let mut i = 0;
            while unsafe { &*next_proc }.state != ProcessState::Ready {
                next_proc = procs.next().expect("no processes to schedule") as *const _ as _;

                // If we checked all others, sleep until an interrupt occurs (will most likely be the CLINT's timer)
                // TODO: This is not a very robust solution at all, it is just a placeholder.
                i += 1;
                if i > procs.processes.len() {
                    unsafe { asm!("wfi") }
                    procs.update_waiting();
                    i = 0;
                }
            }

            unsafe { &mut *next_proc }
        })
        .run()
}
