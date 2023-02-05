#![no_std]
#![no_main]
// Tests dont do anything for now, but this shuts up rust-analyzer
#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]

core::arch::global_asm!(include_str!("./asm/entry.s"));

#[cfg(test)]
fn test_runner(_tests: &[&dyn Fn()]) {}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern "C" {
    fn park();
}

unsafe fn println(s: &str) {
    const UART: *mut u8 = 0x1000_0000 as _;
    for c in s.bytes() {
        UART.write_volatile(c);
    }
    UART.write_volatile(b'\n');
}

#[no_mangle]
extern "C" fn kernel_main() {
    unsafe {
        println("kernel_main() called, we have reached Rust!");
        core::arch::asm!("li t4, 0xFEEDFACECAFEBEEF", "j {park}", park = sym park);
    }
}
