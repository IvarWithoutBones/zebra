pub mod interrupt;
pub mod scheduler;
pub mod syscall;
pub mod trapframe;

use self::trapframe::TrapFrame;
use crate::{
    elf::load_elf,
    memory::{align_page_down, allocator, page, sections::map_trampoline, PAGE_SIZE},
};
use alloc::boxed::Box;
use core::{
    arch::global_asm,
    fmt,
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

extern "C" {
    // Defined in `context_switch.s`
    fn user_enter(trap_frame: *const TrapFrame) -> !;
}

const STACK_SIZE: usize = 40 * PAGE_SIZE;
const TRAPFRAME_ADDR: usize = align_page_down(usize::MAX);
static NEXT_PID: AtomicUsize = AtomicUsize::new(1);

global_asm!(include_str!("context_switch.s"), TRAPFRAME_ADDR = const TRAPFRAME_ADDR);

#[derive(Debug, PartialEq, Eq, Clone)]
enum ProcessState {
    Running,
    Ready,
    WaitUntilMessageReceived,

    Sleeping {
        duration: Duration,
    },

    ChildExited {
        child_pid: usize,
    },

    MessageSent {
        receiver_sid: u64,
    },

    HandlingInterrupt {
        old_state: Box<ProcessState>,
        old_registers: Box<[u64; trapframe::Registers::len()]>,
        interrupt_id: u32,
    },
}

#[repr(C)]
#[derive(Clone)]
pub struct Process {
    state: ProcessState,
    pub pid: usize,
    page_table: Box<page::Table>,
    pub trap_frame: Box<TrapFrame>,
}

impl Process {
    pub fn map_user_stack(page_table: &mut page::Table, size: usize) -> *mut u8 {
        // TODO: guard page
        let user_stack = { allocator().allocate(STACK_SIZE).unwrap() };

        // Map the users stack
        for page in 0..size {
            page_table.map_page(
                user_stack as usize + (page * PAGE_SIZE),
                user_stack as usize + (page * PAGE_SIZE),
                page::EntryAttributes::UserReadWrite,
            );
        }

        unsafe { user_stack.add(size) }
    }

    pub fn new(elf: &[u8]) -> Self {
        let mut page_table = Box::new(page::Table::new());
        // TODO: both stacks desperately need a guard page beneath to catch stack overflows
        let kernel_stack = { allocator().allocate(STACK_SIZE).unwrap() }; // For trapping into the kernel
        let user_stack = Self::map_user_stack(&mut page_table, STACK_SIZE);

        // Map the initialisation code so that we can enter user mode after switching to the new page table
        page_table.identity_map(
            user_enter as usize,
            user_enter as usize + PAGE_SIZE, // TODO: how do know the size of this?
            page::EntryAttributes::ReadExecute,
        );

        map_trampoline(&mut page_table);

        // Map the users program
        let entry = load_elf(elf, &mut page_table);

        let mut trap_frame = TrapFrame::new(page_table.build_satp() as _, user_stack, unsafe {
            kernel_stack.add(STACK_SIZE) // TODO: make this more consistent with the users stack
        });

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
            state: ProcessState::Ready,
            pid: NEXT_PID.fetch_add(1, Ordering::Relaxed),
        }
    }

    pub fn run(&mut self) -> ! {
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
