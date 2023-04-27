#![feature(asm_const, custom_test_frameworks)]
#![reexport_test_harness_main = "test_entry_point"]
#![test_runner(test::test_runner)]
#![no_std]
#![no_main]

#[macro_use]
mod uart;
mod elf;
mod ipc;
mod test;
mod memory;
mod power;
mod process;
mod spinlock;
mod trap;

extern crate alloc;

use core::arch::global_asm;

global_asm!(include_str!("./entry.asm"));

// Until a filesystem is implemented this is good enough for me :^)
const SHELL_ELF: &[u8] = include_bytes!("../../target/riscv64gc-unknown-none-elf/debug/shell");

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

    let proc = process::Process::new(SHELL_ELF);
    println!("\n{proc:#?}\n");
    process::scheduler::insert(proc);

    println!("starting scheduler\n");
    process::scheduler::schedule();
}
