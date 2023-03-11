use {
    crate::memory::{allocator, page, sections::map_trampoline, PAGE_SIZE},
    alloc::boxed::Box,
    core::{arch::global_asm, fmt},
};

extern "C" {
    fn user_enter(trap_frame: *const TrapFrame);
}

const PROGRAM_START: usize = 0x2000_0000;
const USER_STACK_SIZE: usize = 4 * PAGE_SIZE;
const TRAPFRAME_ADDR: usize = 0x1000;

pub static mut TRAPFRAME_PTR: usize = 0;

global_asm!(include_str!("context_switch.s"), TRAPFRAME_ADDR = const TRAPFRAME_ADDR);

#[repr(C)]
#[derive(Default)]
pub struct TrapFrame {
    kernel_satp: u64,
    kernel_trap_vector: u64,
    kernel_stack_pointer: u64,

    satp: u64,
    program_counter: u64,
    stack_pointer: u64,
    return_address: u64,
    global_pointer: u64,
    thread_pointer: u64,

    a0: u64,
    a1: u64,
    a2: u64,
    a3: u64,
    a4: u64,
    a5: u64,
    a6: u64,
    a7: u64,

    t0: u64,
    t1: u64,
    t2: u64,
    t3: u64,
    t4: u64,
    t5: u64,
    t6: u64,

    s0: u64,
    s1: u64,
    s2: u64,
    s3: u64,
    s4: u64,
    s5: u64,
    s6: u64,
    s7: u64,
    s8: u64,
    s9: u64,
}

impl TrapFrame {
    fn new(user_satp: u64, stack_pointer: u64, kernel_stack_pointer: u64) -> Box<Self> {
        Box::new(Self {
            stack_pointer,
            kernel_stack_pointer,
            satp: user_satp,
            program_counter: PROGRAM_START as _,
            ..Default::default()
        })
    }

    const fn as_ptr(&self) -> *const Self {
        self as *const _
    }
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
        let user_stack = { allocator().allocate(USER_STACK_SIZE).unwrap() };
        let kernel_stack = { allocator().allocate(USER_STACK_SIZE).unwrap() };

        let mut page_table = page::Table::new();

        let trap_frame = TrapFrame::new(
            page_table.build_satp() as _,
            unsafe { user_stack.add(USER_STACK_SIZE) } as _,
            unsafe { kernel_stack.add(USER_STACK_SIZE) } as _,
        );

        // Map the trap frame
        page_table.map_page(
            TRAPFRAME_ADDR,
            trap_frame.as_ptr() as usize,
            page::EntryAttributes::ReadWrite,
        );

        // Map the initialisation code so that we can enter user mode after switching to the new page table
        page_table.identity_map(
            user_enter as usize,
            user_enter as usize + PAGE_SIZE, // TODO: how do know the size of this?
            page::EntryAttributes::ReadExecute,
        );

        // Map the users stack
        for page in 0..USER_STACK_SIZE {
            page_table.map_page(
                user_stack as usize + (page * PAGE_SIZE),
                user_stack as usize + (page * PAGE_SIZE),
                page::EntryAttributes::UserReadWrite,
            );
        }

        // Map the users program
        page_table.map_page(
            PROGRAM_START,
            func as usize,
            page::EntryAttributes::UserReadExecute,
        );

        map_trampoline(&mut page_table);

        Self {
            trap_frame,
            state: ProcessState::Waiting,
            page_table: Box::new(page_table),
            pid: 1,
        }
    }

    pub fn run(&mut self) {
        let trap_frame = self.trap_frame.as_ptr();
        unsafe {
            TRAPFRAME_PTR = trap_frame as usize;
            user_enter(trap_frame);
        };
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

pub fn switch_to_process() {
    unsafe {
        user_enter(TRAPFRAME_PTR as *const TrapFrame);
        panic!()
    }
}
