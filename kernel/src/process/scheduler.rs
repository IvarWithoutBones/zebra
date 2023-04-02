use super::{Process, ProcessState};
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
            .find(|p| p.state == ProcessState::WaitingForChild(proc.pid))
            .map(|p| p.state = ProcessState::Waiting)
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

    pub fn handle_sleeping(&mut self) {
        let now = clint::time_since_bootup();
        for proc in self.processes.iter_mut() {
            if let ProcessState::Sleeping(until) = proc.state {
                if now >= until {
                    proc.state = ProcessState::Waiting;
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
                    current.state = ProcessState::Waiting;
                }
            }

            let mut next_proc: *mut Process =
                procs.next().expect("no processes to schedule") as *const _ as _;

            // Find a process that is waiting to run
            let mut i = 0;
            while unsafe { &*next_proc }.state != ProcessState::Waiting {
                next_proc = procs.next().expect("no processes to schedule") as *const _ as _;

                // If we checked all others, sleep until an interrupt occurs (will most likely be the CLINT's timer)
                // TODO: This is not a very robust solution at all, it is just a placeholder.
                if i >= procs.processes.len() {
                    unsafe { asm!("wfi") }
                    procs.handle_sleeping();
                    i = 0;
                }
                i += 1;
            }

            unsafe { &mut *next_proc }
        })
        .run()
}
