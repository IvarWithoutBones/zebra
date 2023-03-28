#![feature(custom_test_frameworks)]
#![test_runner(librs::language_items::test_runner)]
#![no_std]
#![no_main]

librs::main!(main);

fn main() {
    println!("hello from seperate crate");
}
