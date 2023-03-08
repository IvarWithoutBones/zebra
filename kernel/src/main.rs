#![feature(panic_info_message, custom_test_frameworks, fn_align)]
#![reexport_test_harness_main = "test_entry_point"]
#![test_runner(language_items::test_runner)]
#![no_std]
#![no_main]

#[macro_use]
mod language_items;
mod memory;
mod power;
mod process;
mod spinlock;
mod trap;
mod uart;

extern crate alloc;

use {
    arbitrary_int::u3,
    core::arch::{asm, global_asm},
};

global_asm!(include_str!("./asm/entry.s"));

#[repr(align(4096))]
fn user_func() {
    unsafe {
        asm!("li t0, 0xdeadbeef");

        #[allow(unused_variables)]
        let mut i = 0;

        loop {
            i += 1;
        }
    }
}

#[no_mangle]
extern "C" fn kernel_main() {
    uart::UART.lock_with(|uart| uart.init());

    unsafe {
        trap::attach_supervisor_trap_vector();
        memory::init();
        trap::plic::set_global_threshold(u3::new(5));
        trap::plic::add_device::<uart::NS16550a>();
        // TODO: breaks because we never switch back from the users page table
        // trap::enable_interrupts();
    }

    let mut proc = process::Process::new(user_func);
    println!("{proc:?}");
    proc.run();

    // Start executing the reexported test harness's entry point.
    // This will shut down the system when testing is complete.
    #[cfg(test)]
    test_entry_point();

    loop {
        unsafe { asm!("wfi") }
    }
}
