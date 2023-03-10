use {
    crate::memory::{
        allocator, page,
        sections::{map_trampoline, trampoline_start},
        PAGE_SIZE,
    },
    alloc::boxed::Box,
    core::{arch::global_asm, fmt},
};

extern "C" {
    fn user_enter(trap_frame: usize, trampoline: usize);
}

const PROGRAM_START: usize = 0x2000_0000;
const USER_STACK_PAGES: usize = 4 * PAGE_SIZE;
const TRAPFRAME_ADDR: usize = 0x1000;

global_asm!(include_str!("context_switch.s"), TRAPFRAME_ADDR = const TRAPFRAME_ADDR);

#[repr(C)]
#[derive(Default)]
struct TrapFrame {
    user_satp: usize,
    kernel_satp: usize,
    kernel_trap_vector: usize,
    program_counter: usize,
    stack_pointer: usize,
}

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
    page_table: Box<page::Table>,
    trap_frame: Box<TrapFrame>,
}

impl Process {
    pub fn new(func: fn()) -> Self {
        let stack = { allocator().allocate(USER_STACK_PAGES).unwrap() };
        let mut page_table = page::Table::new();

        let mut trap_frame = Box::<TrapFrame>::default();
        trap_frame.user_satp = page_table.build_satp();
        trap_frame.program_counter = PROGRAM_START;
        trap_frame.stack_pointer = unsafe { stack.add(PAGE_SIZE * USER_STACK_PAGES) } as _;

        // Map the initialisation code so that we can enter user mode after switching to the new page table
        page_table.identity_map(
            user_enter as usize,
            user_enter as usize + PAGE_SIZE, // TODO: how do know the size of this?
            page::EntryAttributes::ReadExecute,
        );

        // Map the users stack
        for page in 0..USER_STACK_PAGES {
            page_table.map_page(
                stack as usize + (PAGE_SIZE * page),
                stack as usize + (PAGE_SIZE * page),
                page::EntryAttributes::UserReadWrite,
            );
        }

        // Map the users program
        page_table.map_page(
            PROGRAM_START,
            func as usize,
            page::EntryAttributes::UserReadExecute,
        );

        // Map the trampoline
        map_trampoline(&mut page_table);

        // Map the trap frame
        page_table.map_page(
            TRAPFRAME_ADDR,
            trap_frame.as_mut() as *mut _ as usize,
            page::EntryAttributes::ReadWrite,
        );

        Self {
            state: ProcessState::Waiting,
            pid: 0,
            page_table: Box::new(page_table),
            trap_frame,
        }
    }

    pub fn run(&mut self) {
        unsafe {
            user_enter(
                self.trap_frame.as_mut() as *mut _ as usize,
                trampoline_start(),
            );
        }
    }
}

impl Drop for TrapFrame {
    fn drop(&mut self) {
        allocator().deallocate(self.stack_pointer as _);
    }
}

impl fmt::Debug for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Process")
            .field("state", &self.state)
            .field("pid", &self.pid)
            .field("page_table", &(&self.page_table as *const _))
            .field("trap_frame", &(&self.trap_frame as *const _))
            .finish_non_exhaustive()
    }
}
