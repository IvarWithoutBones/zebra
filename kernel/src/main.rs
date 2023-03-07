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

fn user_test() {
    unsafe {
        asm!("li t5, 0xdeadbeef");
        asm!("li t4, 0xfacecaf");

        let mut count = 0;
        for character in b"Hello from user land!\r\n".iter() {
            // Write to the UART
            asm!("sd {}, 0({})", in(reg) *character, in(reg) uart::BASE_ADDR);
            count += 1;
        }

        // Just to confirm we can use the stack (praying this doesnt get optimized out)
        char::from_digit(count, 10).unwrap_or('0');

        asm!("ecall");

        // There is no `wfi` for user mode
        #[allow(clippy::empty_loop)]
        loop {}
    };
}

#[no_mangle]
extern "C" fn kernel_main() {
    uart::UART.lock_with(|uart| uart.init());

    unsafe {
        trap::attach_supervisor_trap_vector();
        memory::init();
        trap::plic::set_global_threshold(u3::new(0));
        trap::plic::add_device::<uart::NS16550a>();
        trap::enable_interrupts();
    }

    let mut proc = process::Process::new(user_test);
    println!("process created: {:?}", proc);
    proc.run();

    // Start executing the reexported test harness's entry point.
    // This will shut down the system when testing is complete.
    #[cfg(test)]
    test_entry_point();

    loop {
        unsafe { asm!("wfi") }
    }
}
