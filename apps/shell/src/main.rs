#![feature(custom_test_frameworks)]
#![test_runner(librs::language_items::test_runner)]
#![no_std]
#![no_main]

librs::main!(main);

use alloc::{string::String, vec::Vec};
use core::time::Duration;
use librs::syscall;

// Filesystems are bloatware
const HELLO_ELF: &[u8] = include_bytes!("../../../target/riscv64gc-unknown-none-elf/debug/hello");
const USERNAME: &str = "someone";

const SLEEP_DURATION: Duration = Duration::from_millis(20);

fn print_prefix() {
    print!("[{USERNAME}@zebra:~]$ ");
}

fn handle_command(line: &str) {
    let mut iter = line.trim().split_ascii_whitespace();
    let command = iter.next().unwrap_or("");
    println!(); // Newline

    match command {
        "exit" => syscall::exit(),

        "uptime" => {
            let uptime = syscall::duration_since_boot();
            println!("uptime: {uptime:?}");
        }

        "hello" => {
            syscall::spawn(HELLO_ELF);
            // Dont mess up the prompt. TODO: a blocking `syscall::spawn()`
            syscall::sleep(Duration::from_millis(100));
        }

        "sleep" => {
            let secs: u64 = iter.next().unwrap().parse().unwrap();
            let duration = Duration::from_secs(secs);
            println!("sleeping for {secs} seconds");
            syscall::sleep(duration);
            println!("done sleeping");
        }

        "echo" => {
            let args = iter.collect::<Vec<_>>().join(" ");
            println!("{args}");
        }

        // Not to pat myself on the back too much but modern shells can learn a thing or two from this
        "whoami" => println!("{USERNAME}"),
        "uname" => println!("Zebra"),
        "ls" => println!("Downloads Documents Pictures Music Videos foo.sh"),
        "./foo.sh" => println!("hello world"),
        "cat foo.sh" => println!("#!/usr/bin/bash\necho hello world"),

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
                // Enter
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

                // Control-D
                '\x04' => {
                    handle_command("exit");
                }

                // Regular character
                _ if c.is_ascii_alphanumeric() || c.is_ascii_punctuation() || c == ' ' => {
                    print!("{c}");
                    command.push(c);
                }

                _ => {}
            }
        } else {
            syscall::sleep(SLEEP_DURATION);
        }
    }
}