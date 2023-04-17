#![feature(custom_test_frameworks)]
#![test_runner(librs::test::test_runner)]
#![no_std]
#![no_main]

librs::main!(main);

use alloc::{string::String, vec::Vec};
use core::time::Duration;
use librs::syscall;

// Filesystems are bloatware
mod elfs {
    #![allow(dead_code)]

    pub const HELLO: &[u8] =
        include_bytes!("../../../target/riscv64gc-unknown-none-elf/debug/hello");
    pub const LOG: &[u8] =
        include_bytes!("../../../target/riscv64gc-unknown-none-elf/debug/log-server");
    pub const USTAR: &[u8] =
        include_bytes!("../../../target/riscv64gc-unknown-none-elf/debug/ustar");
    pub const VIRTIO: &[u8] =
        include_bytes!("../../../target/riscv64gc-unknown-none-elf/debug/virtio");
}

const SLEEP_DURATION: Duration = Duration::from_millis(20);

fn print_prefix() {
    print!("$ ");
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
            syscall::spawn(elfs::HELLO, true);
        }

        "async_hello" => {
            syscall::spawn(elfs::HELLO, false);
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
        "uname" => println!("Zebra"),
        "ls" => println!("Downloads Documents Pictures Music Videos foo.sh"),
        "./foo.sh" => println!("hello world"),
        "cat foo.sh" => println!("#!/usr/bin/bash\necho hello world"),

        "" => {}
        _ => println!("unknown command: {line}"),
    }
}

fn main() {
    syscall::register_server(None);

    // Until an `init` process exists
    syscall::spawn(elfs::LOG, false);
    syscall::sleep(SLEEP_DURATION); // Dont print before the log server is set up

    syscall::spawn(elfs::VIRTIO, false);
    syscall::sleep(SLEEP_DURATION);
    syscall::spawn(elfs::USTAR, true);

    println!("welcome to knockoff bash");
    print_prefix();

    let mut command = String::with_capacity(0x100);

    loop {
        if let Some(b) = log_server::read() {
            let c = b as char;
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
