#![feature(custom_test_frameworks)]
#![test_runner(test::test_runner)]
#![reexport_test_harness_main = "test_entry_point"]
#![no_std]

#[macro_use]
pub mod print;
pub mod allocator;
pub mod syscall;
pub mod test;

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

extern "C" {
    fn __zebra_main();
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
