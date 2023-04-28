use crate::{
    memory::{self, align_page_down, page, PAGE_SIZE},
    spinlock::SpinLock,
};
use alloc::{boxed::Box, fmt};
use core::pin::Pin;

pub mod context;

// Defined in `switch.asm`
extern "C" {
    pub fn switch_into(trap_frame: *const context::TrapFrame) -> !;
    pub fn user_trap_vector();
}

const TRAPFRAME_PTR: usize = align_page_down(usize::MAX);
const KERNEL_STACK_SIZE: usize = PAGE_SIZE;
const USER_STACK_SIZE: usize = 5 * PAGE_SIZE;

static THREAD_CONTROLLER: SpinLock<Option<ThreadController>> = SpinLock::new(None);

#[derive(Debug)]
pub struct ThreadController {
    pub thread: Thread,
    trap_handler: Option<*const fn()>,
}

impl ThreadController {
    pub const fn new(thread: Thread) -> Self {
        Self {
            thread,
            trap_handler: None,
        }
    }
}

pub fn handle_trap(cause: u64) {
    let controller: &mut ThreadController = unsafe { &mut *(thread_controller() as *mut _) };
    if controller.thread.trap_frame.user_state[context::Registers::A0] == 1
        && controller.trap_handler.is_none()
    {
        attach_trap_handler(controller);
    }

    if let Some(trap_handler) = controller.trap_handler {
        controller.thread.trap_frame.user_state[context::Registers::ProgramCounter] =
            trap_handler as _;
        controller.thread.trap_frame.user_state[context::Registers::A0] = cause;
    }

    unsafe { controller.thread.switch_into() };
}

fn attach_trap_handler(controller: &mut ThreadController) {
    assert!(controller.trap_handler.is_none());
    assert_eq!(
        controller.thread.trap_frame.user_state[context::Registers::A0],
        1
    );

    controller.trap_handler =
        Some(controller.thread.trap_frame.user_state[context::Registers::A1] as *const fn());
}

pub fn set_thread_controller(thread: Thread) {
    THREAD_CONTROLLER.lock_with(|controller| {
        *controller = Some(ThreadController::new(thread));
    });
}

pub unsafe fn thread_controller() -> *const ThreadController {
    THREAD_CONTROLLER.lock_with(|controller| controller.as_ref().unwrap() as _)
}

pub struct Thread {
    pub trap_frame: Box<context::TrapFrame>,
    pub page_table: Box<page::Table>,
    user_stack: Pin<Box<[u8; USER_STACK_SIZE]>>,
    kernel_stack: Pin<Box<[u8; KERNEL_STACK_SIZE]>>,
}

impl Thread {
    pub fn new() -> Self {
        let kernel_stack = Box::pin([0; KERNEL_STACK_SIZE]);
        let user_stack = Box::pin([0; USER_STACK_SIZE]);
        let mut page_table = Box::new(page::Table::new());

        let mut trap_frame = unsafe {
            context::TrapFrame::new(
                kernel_stack.as_ptr().add(kernel_stack.len()),
                user_stack.as_ptr().add(user_stack.len()),
            )
        };

        // Map the trampoline so that we return to the kernel after a trap.
        memory::sections::map_trampoline(&mut page_table);

        // Map the entry function so that we can continue with it after switching to the target page table.
        page_table.identity_map(
            switch_into as usize,
            switch_into as usize + PAGE_SIZE, // TODO: how do we know the size of this?
            page::EntryAttributes::ReadExecute,
        );

        // Map the trapframe for the user so that we can store context upon a trap.
        page_table.map_page(
            TRAPFRAME_PTR,
            trap_frame.as_ptr() as _,
            page::EntryAttributes::ReadWrite,
        );

        // Map the trapframe for the kernel so that we can store context upon a trap.
        page::root_table().map_page(
            TRAPFRAME_PTR,
            trap_frame.as_ptr() as _,
            page::EntryAttributes::ReadWrite,
        );

        for page in 0..=memory::pages_needed(user_stack.len()) {
            let page_addr = user_stack.as_ptr() as usize + (page * PAGE_SIZE);
            println!("mapping user stack page {:x}", page_addr);
            page_table.map_page(page_addr, page_addr, page::EntryAttributes::UserReadWrite);
        }

        trap_frame.set_user_satp(page_table.build_satp() as _);

        Self {
            trap_frame,
            page_table,
            kernel_stack,
            user_stack,
        }
    }

    pub unsafe fn switch_into(&self) -> ! {
        self.trap_frame.run();
    }
}

impl fmt::Debug for Thread {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Thread")
            .field("trap_frame", &self.trap_frame)
            .field("page_table", &(&self.page_table as *const _))
            .field("user_stack", &self.user_stack.as_ptr())
            .field("kernel_stack", &self.kernel_stack.as_ptr())
            .finish()
    }
}
