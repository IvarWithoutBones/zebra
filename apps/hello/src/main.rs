#![feature(custom_test_frameworks)]
#![test_runner(librs::language_items::test_runner)]
#![no_std]
#![no_main]

librs::main!(main);

fn main() {
    // Extra newline is just here until proper standard output synchronization is implemented, makes things look nicer
    println!("Hello, world!");
}
