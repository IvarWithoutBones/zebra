#![feature(custom_test_frameworks)]
#![test_runner(librs::language_items::test_runner)]
#![no_std]
#![no_main]

librs::main!(main);

use librs::syscall;

const USERNAME: &str = "someone";

fn print_prefix() {
    print!("[{USERNAME}@zebra:~]$ ");
}

fn handle_command(line: &str) {
    let line = line.trim();
    println!(); // Newline

    // Not to pat myself on the back too mmuch but modern shells can learn a thing or two from this
    match line {
        "exit" => syscall::exit(),
        "whoami" => println!("{USERNAME}"),
        "uname" => println!("Zebra"),
        "ls" => println!("Downloads Documents Pictures Music Videos foo.sh"),
        "cat foo.sh" => println!("#!/usr/bin/bash\necho hello world"),
        "./foo.sh" => println!("hello world"),
        "echo hello world" => println!("hello world"),

        "" => {}
        _ => println!("unknown command: {line}"),
    }
}

fn main() {
    println!("welcome to knockoff bash");
    print_prefix();

    // TODO: Implement a userland dynamic allocator so that we can just use a String here.
    // Should also add a standardized function like `librs::io::read_line()` matching the standard library.
    let mut buffer = [0u8; 0x1000];
    let mut start_offset = 0;
    let mut index = 0;
    let mut at_beginning = true;

    loop {
        if let Some(c) = syscall::read() {
            buffer[index] = c as u8;

            match c {
                '\r' => {
                    buffer[index] = b'\0';
                    let string = core::str::from_utf8(&buffer[start_offset..index]);
                    if let Err(err) = string {
                        println!("INTERNAL ERROR: {err}");
                    } else {
                        handle_command(string.unwrap());
                    }

                    index = 0;
                    start_offset = 0;
                    at_beginning = true;
                    print_prefix();
                }

                // Backspace
                '\x7f' if !at_beginning => {
                    index = index.saturating_sub(1);
                    start_offset = start_offset.saturating_sub(1);
                    print!("\x08 \x08");

                    if index - start_offset == 0 {
                        at_beginning = true;
                        start_offset = 0;
                        index = 0;
                    }
                }

                _ if c.is_ascii_alphanumeric() || c.is_ascii_punctuation() || c == ' ' => {
                    at_beginning = false;
                    print!("{c}");

                    if index < buffer.len() - 1 {
                        index += 1;
                    } else {
                        println!("\nbuffer full, discarding");
                        index = 0;
                        start_offset = 0;
                        at_beginning = true;
                        print_prefix();
                    }
                }

                _ => {}
            }
        }
    }
}
