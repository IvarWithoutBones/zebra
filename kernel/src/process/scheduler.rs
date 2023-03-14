use {
    super::{Process, ProcessState},
    crate::spinlock::Spinlock,
    alloc::vec::Vec,
};

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
        Some(self.processes.remove(self.current?))
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
}

pub fn insert(process: Process) {
    PROCESSES.lock_with(|processes| processes.push(process));
}

pub fn schedule() -> ! {
    // We need a reference to the process that remains valid *after* dropping the PROCESSES lock,
    // should probably use a smart pointer instead of the unsafe raw pointer.
    let next_proc = PROCESSES.lock_with(|procs| {
        if let Some(current) = procs.current() {
            current.state = ProcessState::Waiting;
        }

        let next_proc: *mut Process =
            procs.next().expect("no processes to schedule") as *const _ as _;
        unsafe { &mut *next_proc }
    });

    println!("switching to pid {}", next_proc.pid);
    next_proc.run()
}
