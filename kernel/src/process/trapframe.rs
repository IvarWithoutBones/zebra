use {super::PROGRAM_START, alloc::boxed::Box, core::fmt};

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
    pub fn new(user_satp: u64, stack_pointer: u64, kernel_stack_pointer: u64) -> Box<Self> {
        Box::new(Self {
            stack_pointer,
            kernel_stack_pointer,
            satp: user_satp,
            program_counter: PROGRAM_START as _,
            ..Default::default()
        })
    }

    pub const fn as_ptr(&self) -> *const Self {
        self as *const _
    }
}

impl fmt::Debug for TrapFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TrapFrame")
            .field("pointer", &self.as_ptr())
            .field("user_satp", &format_args!("{:#X}", self.satp))
            .field(
                "program_counter",
                &format_args!("{:#X}", self.program_counter),
            )
            .finish_non_exhaustive()
    }
}
