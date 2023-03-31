#![feature(custom_test_frameworks)]
#![test_runner(librs::language_items::test_runner)]
#![no_std]
#![no_main]

librs::main!(main);

use alloc::{string::String, vec::Vec};
use librs::syscall;

const USERNAME: &str = "someone";

fn print_prefix() {
    print!("[{USERNAME}@zebra:~]$ ");
}

fn handle_command(line: &str) {
    let mut iter = line.trim().split_ascii_whitespace();
    let command = iter.next().unwrap_or("");
    println!(); // Newline

    // Not to pat myself on the back too mmuch but modern shells can learn a thing or two from this
    match command {
        "exit" => syscall::exit(),
        "whoami" => println!("{USERNAME}"),
        "uname" => println!("Zebra"),
        "ls" => println!("Downloads Documents Pictures Music Videos foo.sh"),
        "./foo.sh" => println!("hello world"),
        "cat foo.sh" => println!("#!/usr/bin/bash\necho hello world"),

        "echo" => {
            let args = iter.collect::<Vec<_>>().join(" ");
            println!("{args}");
        }

        "" => {}
        _ => println!("unknown command: {line}"),
    }
}

fn main() {
    println!("welcome to knockoff bash");
    print_prefix();

    let mut command = String::with_capacity(0x100);

    loop {
        if let Some(c) = syscall::read() {
            match c {
                '\r' => {
                    handle_command(&command);
                    print_prefix();
                    command.clear();
                }

                // Backspace
                '\x7f' if !command.is_empty() => {
                    print!("\x08 \x08");
                    command.pop();
                }

                _ if c.is_ascii_alphanumeric() || c.is_ascii_punctuation() || c == ' ' => {
                    print!("{c}");
                    command.push(c);
                }

                _ => {}
            }
        }
    }
}
