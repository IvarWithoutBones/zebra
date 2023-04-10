#![feature(custom_test_frameworks)]
#![test_runner(test_runner)]
#![no_std]

///! Resources:
///! https://en.wikipedia.org/wiki/Executable_and_Linkable_Format
///! https://upload.wikimedia.org/wikipedia/commons/e/e4/ELF_Executable_and_Linkable_Format_diagram_by_Ange_Albertini.png
///! https://man7.org/linux/man-pages/man5/elf.5.html
///! https://docs.oracle.com/cd/E23824_01/html/819-0690/chapter6-46512.html#scrolltoc
///! https://wiki.osdev.org/ELF_Tutorialhttps://wiki.osdev.org/ELF_Tutorial
///! $ cargo readobj -- --headers
pub mod header;
pub mod program;
pub mod section;
pub mod symbol;

extern crate alloc;

#[cfg(test)]
fn test_runner(tests: &[&dyn Fn()]) {
    tests.iter().for_each(|test| test());
}
