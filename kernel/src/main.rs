#![feature(asm_const, custom_test_frameworks)]
#![reexport_test_harness_main = "test_entry_point"]
#![test_runner(test::test_runner)]
#![no_std]
#![no_main]
// TODO: remove together with legacy process API once the userland scheduler is functional
#![allow(dead_code, unused_variables)]

#[macro_use]
mod uart;
mod elf;
// mod ipc;
mod memory;
mod power;
// mod process;
mod spinlock;
mod test;
mod thread;
mod trap;

extern crate alloc;

use core::arch::global_asm;

global_asm!(include_str!("./entry.asm"));

// Until a filesystem is implemented this is good enough for me :^)
const INIT_ELF: &[u8] = include_bytes!("../../target/riscv64gc-unknown-none-elf/debug/init");

#[no_mangle]
extern "C" fn kernel_main() {
    uart::UART.lock_with(|uart| uart.init());

    unsafe {
        trap::attach_supervisor_trap_vector();
        memory::init();
        trap::plic::set_global_threshold(0);

        // No needs for interrupts in non-integration tests
        #[cfg(not(test))]
        {
            println!("enabling interrupts...");
            trap::enable_interrupts();
            println!("interrupts enabled");
        }
    }

    // Start executing the reexported test harness's entry point.
    // This will shut down the system when testing is complete.
    #[cfg(test)]
    test_entry_point();

    let mut thread = thread::Thread::new();
    let entry_point = elf::load_elf(INIT_ELF, &mut thread.page_table);
    println!("entry point: {entry_point:#x}");
    thread.trap_frame.user_state[thread::context::Registers::ProgramCounter] = entry_point;
    println!("trap frame: {thread:#?}\nrunning");

    thread.page_table.identity_map(
        uart::BASE_ADDR as usize,
        uart::BASE_ADDR as usize + 5,
        memory::page::EntryAttributes::UserReadWrite,
    );

    memory::allocator().disable();
    thread::set_thread_controller(thread);
    unsafe { (*thread::thread_controller()).thread.switch_into() };
}
