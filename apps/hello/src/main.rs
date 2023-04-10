#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]
#![no_main]

librs::main!(main);

fn main() {
    librs::syscall::register_server(None);
    println!("Hello, world!");
}
