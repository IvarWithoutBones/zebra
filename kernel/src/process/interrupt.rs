use super::{scheduler, trapframe::Registers, Process, ProcessState};
use alloc::boxed::Box;

/// Handle an external interrupt for the given process, by context switching into its designated handler.
/// The signature of this function must match the definition in `plic.s`
pub fn handle(interrupt_id: u32, handler_ptr: usize, pid: usize) {
    let proc = scheduler::PROCESSES.lock_with(|procs| {
        // Update the state of the previously running process
        if let Some(curr) = procs.processes.front_mut() {
            if curr.state == ProcessState::Running {
                curr.state = ProcessState::Ready;
            }
        }

        let proc = procs.find_by_pid(pid).unwrap();

        if let ProcessState::HandlingInterrupt { .. } = proc.state {
            // TODO: dont just drop the current interrupt
            return None;
        }

        let old_state = Box::new(proc.state.clone());
        let old_registers = Box::new(proc.trap_frame.registers);

        // Stash away the old state so that we can restore it when the interrupt handler returns
        proc.state = ProcessState::HandlingInterrupt {
            old_state,
            old_registers,
            interrupt_id,
        };

        // Ensure we dont depend on any previous state (except `SATP`)
        proc.trap_frame.registers[Registers::ProgramCounter as _..].fill(0);

        // Allocate a new stack for the interrupt handler
        let new_stack = Process::map_user_stack(&mut proc.page_table);
        proc.trap_frame.registers[Registers::StackPointer as usize] = new_stack as _;

        // Start execution at the interrupt handler
        proc.trap_frame.registers[Registers::ProgramCounter as usize] = handler_ptr as _;

        // Bypass the borrow checker so that we can release the processes lock
        let proc = proc as *mut Process;
        Some(unsafe { &mut *proc })
    });

    if let Some(proc) = proc {
        proc.run();
    }
}
