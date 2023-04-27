#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![no_std]
#![no_main]

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    tests.iter().for_each(|test| test());
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    print_str("error: panicked\n");
    loop {}
}

// TODO: use the `uart` crate
fn print_char(c: char) {
    // TODO: capability based identity mapping
    unsafe { (0x10000000 as *mut u8).write_volatile(c as _) }
}

fn print_str(s: &str) {
    for c in s.bytes() {
        print_char(c as _)
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    print_str("hello world\n");

    unsafe {
        // TODO: proper syscall API
        core::arch::asm!("ecall", in("a0") 1, in("a1") signal_handler as usize);
    }

    #[allow(clippy::empty_loop)]
    loop {}
}

extern "C" fn signal_handler(cause: u64) {
    print_str("signal received. ");
    let cause_str = core::char::from_digit(cause as _, 10).unwrap();
    print_str("cause: ");
    print_char(cause_str);
    print_str("\n");
}
