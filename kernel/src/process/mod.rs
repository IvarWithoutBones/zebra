pub mod scheduler;
pub mod syscall;
pub mod trapframe;

use self::trapframe::TrapFrame;
use crate::memory::{allocator, page, sections::map_trampoline, PAGE_SIZE};
use alloc::boxed::Box;
use core::{
    arch::global_asm,
    fmt,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

const STACK_SIZE: usize = 40 * PAGE_SIZE;
const TRAPFRAME_ADDR: usize = 0x1000;

static NEXT_PID: AtomicUsize = AtomicUsize::new(1);

global_asm!(include_str!("context_switch.s"), TRAPFRAME_ADDR = const TRAPFRAME_ADDR);

extern "C" {
    // Defined in `context_switch.s`
    fn user_enter(trap_frame: *const TrapFrame) -> !;
}

#[derive(Debug, PartialEq, Eq)]
enum ProcessState {
    Running,
    Waiting,
    // TODO: does it make sense to merge the two below, with it holding a more generic `WaitCondition`?
    Sleeping(Duration),
    WaitingForChild(usize),
}

#[repr(C)]
pub struct Process {
    state: ProcessState,
    pub pid: usize,
    page_table: Box<page::Table>,
    pub trap_frame: Box<TrapFrame>,
}

impl Process {
    pub fn new(elf: &[u8]) -> Self {
        let user_stack = { allocator().allocate(STACK_SIZE).unwrap() };
        // Used when trapping into the kernel, desperately needs a guard page.
        let kernel_stack = { allocator().allocate(STACK_SIZE).unwrap() };
        let mut page_table = Box::new(page::Table::new());

        // Map the initialisation code so that we can enter user mode after switching to the new page table
        page_table.identity_map(
            user_enter as usize,
            user_enter as usize + PAGE_SIZE, // TODO: how do know the size of this?
            page::EntryAttributes::ReadExecute,
        );

        // Map the users stack
        for page in 0..STACK_SIZE {
            page_table.map_page(
                user_stack as usize + (page * PAGE_SIZE),
                user_stack as usize + (page * PAGE_SIZE),
                page::EntryAttributes::UserReadWrite,
            );
        }

        map_trampoline(&mut page_table);

        // Map the users program
        let entry = crate::fairy::load_elf(elf, &mut page_table);

        let mut trap_frame = TrapFrame::new(
            page_table.build_satp() as _,
            unsafe { user_stack.add(STACK_SIZE) } as _,
            unsafe { kernel_stack.add(STACK_SIZE) } as _,
        );

        trap_frame.registers[trapframe::Registers::ProgramCounter as usize] = entry;

        // Map the trap frame
        page_table.map_page(
            TRAPFRAME_ADDR,
            trap_frame.as_ptr() as usize,
            page::EntryAttributes::ReadWrite,
        );

        Self {
            trap_frame,
            page_table,
            state: ProcessState::Waiting,
            pid: NEXT_PID.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn run(&mut self) -> ! {
        self.state = ProcessState::Running;
        unsafe { user_enter(self.trap_frame.as_ptr()) }
    }
}

impl fmt::Debug for Process {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Process")
            .field("pid", &self.pid)
            .field("state", &self.state)
            .field("page_table", &(&self.page_table as *const _))
            .field("trap_frame", &self.trap_frame)
            .finish_non_exhaustive()
    }
}
