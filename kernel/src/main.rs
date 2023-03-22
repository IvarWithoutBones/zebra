#![feature(asm_const, custom_test_frameworks, fn_align, panic_info_message)]
#![reexport_test_harness_main = "test_entry_point"]
#![test_runner(language_items::test_runner)]
#![no_std]
#![no_main]

#[macro_use]
mod language_items;
mod fairy;
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
        let mut a = false;
        let mut i: u8 = 0;

        loop {
            if i == u8::MAX {
                if a {
                    asm!("li t0, 0xdeadbeef");
                } else {
                    asm!("li t0, 0xcafeface");
                }

                a = !a;
            }

            i = i.wrapping_add(1);
        }
    }
}

#[repr(align(4096))]
fn user_func_two() {
    unsafe {
        let mut i = 0;
        loop {
            asm!("li a7, 2"); // SystemCall::Print
            asm!("ecall", in("a0") b"a"[0] + i);

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
        trap::plic::set_global_threshold(u3::new(0));
        trap::plic::add_device::<uart::NS16550a>();

        // No needs for interrupts in non-integration tests
        #[cfg(not(test))]
        trap::enable_interrupts();
    }

    // Start executing the reexported test harness's entry point.
    // This will shut down the system when testing is complete.
    #[cfg(test)]
    test_entry_point();

    let proc = process::Process::new(user_func);
    println!("\n{proc:#?}\n");
    process::scheduler::insert(proc);

    let proc_two = process::Process::new(user_func_two);
    println!("\n{proc_two:#?}\n");
    process::scheduler::insert(proc_two);

    println!("starting scheduler");
    process::scheduler::schedule();
}
