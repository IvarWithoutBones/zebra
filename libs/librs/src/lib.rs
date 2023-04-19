#![feature(custom_test_frameworks)]
#![test_runner(test::test_runner)]
#![reexport_test_harness_main = "test_entry_point"]
#![no_std]

#[macro_use]
pub mod print;
pub mod allocator;
pub mod ipc;
pub mod mutex;
pub mod path;
pub mod syscall;
pub mod test;

extern crate alloc;

use core::arch::asm;

pub const PAGE_SIZE: usize = 4096;

#[inline]
pub fn memory_sync() {
    unsafe {
        asm!("fence");
    }
}

#[inline]
pub fn align_page_up(size: usize) -> usize {
    let remainder = size % PAGE_SIZE;
    if remainder == 0 {
        size
    } else {
        (size + PAGE_SIZE) - remainder
    }
}

extern "C" {
    fn __zebra_main();
}

/// Defines the entry point of the program, which is called by the `librs` runtime.
#[macro_export]
macro_rules! main {
    ($func:expr) => {
        #[macro_use]
        extern crate librs;
        extern crate alloc;

        /// The entry point of program execution, called by the `librs` runtime.
        #[no_mangle]
        pub extern "C" fn __zebra_main() {
            // Type check the function
            let function: fn() = $func;
            function();
        }
    };
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    #[cfg(test)]
    test_entry_point();

    unsafe {
        __zebra_main();
    }

    syscall::exit();
}
