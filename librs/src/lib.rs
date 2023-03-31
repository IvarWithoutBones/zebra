#![feature(custom_test_frameworks)]
#![test_runner(language_items::test_runner)]
#![reexport_test_harness_main = "test_entry_point"]
#![no_std]

#[macro_use]
pub mod language_items;
pub mod allocator;
pub mod syscall;

extern "C" {
    fn __zebra_main();
}

/// Defines the entry point of the program, which is called by the ELF loader.
#[macro_export]
macro_rules! main {
    ($func:expr) => {
        #[macro_use]
        extern crate librs;
        extern crate alloc;

        /// The entry point of the program, called by the ELF loader.
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
