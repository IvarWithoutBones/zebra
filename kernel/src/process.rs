use {
    crate::memory::{allocator, page, PAGE_SIZE},
    alloc::boxed::Box,
    core::fmt::Debug,
};

const PROGRAM_START: usize = 0x2000_0000;
const STACK_PAGES: usize = 4;

#[allow(dead_code)]
#[derive(Debug)]
enum ProcessState {
    Running,
    Sleeping,
    Waiting,
    Dead,
}

#[repr(C)]
pub struct Process {
    state: ProcessState,
    pid: usize,
    stack: *mut u8,
    pub program_counter: usize,
    page_table: Box<page::Table>,
}

extern "C" {
    fn user_enter(pc: *const u8, sp: *const u8, satp: usize);
}

impl Process {
    pub fn new(func: fn()) -> Self {
        let stack = { allocator().allocate(PAGE_SIZE).unwrap() };
        let mut page_table = page::Table::new();

        // Map the initialisation code so that we can enter user mode after switching to the new page table
        page_table.identity_map(
            user_enter as usize,
            user_enter as usize + PAGE_SIZE, // TODO: how do we calculate this?
            page::EntryAttributes::ReadExecute as _,
        );

        // Map the user stack
        // TODO: seems to be broken
        page_table.identity_map(
            stack as usize,
            stack as usize + (PAGE_SIZE * STACK_PAGES),
            page::EntryAttributes::UserReadWrite as _,
        );

        // Map the user program
        page_table.user_map(
            PROGRAM_START,
            func as usize,
            page::EntryAttributes::UserReadExecute as _,
        );

        Self {
            state: ProcessState::Waiting,
            pid: 0,
            stack,
            program_counter: PROGRAM_START,
            page_table: Box::new(page_table),
        }
    }

    pub fn run(&mut self) {
        unsafe {
            user_enter(
                self.program_counter as _,
                self.stack,
                self.page_table.build_satp(),
            );
        }
    }
}

impl Drop for Process {
    fn drop(&mut self) {
        allocator().deallocate(self.stack);
    }
}

impl Debug for Process {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Process")
            .field("state", &self.state)
            .field("pid", &self.pid)
            .field("stack", &(&self.stack as *const _))
            .field("program_counter", &(&self.program_counter as *const _))
            .field("page_table", &(&self.page_table as *const _))
            .finish_non_exhaustive()
    }
}
