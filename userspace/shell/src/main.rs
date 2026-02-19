//! Simple Shell
//!
//! Basic command-line interface for the SIGRUN operating system.

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate common;
extern crate syscall_api;

pub mod interpreter;

use interpreter::Parser;
use syscall_api::{SyscallArgs, SYSCALL_READ, SYSCALL_WRITE};

pub fn main() -> ! {
    println("SIGRUN Shell v0.1");
    println("Type 'help' for commands\n");
    
    let mut shell = Shell::new();
    shell.run()
}

struct Shell {
    current_dir: [u8; 256],
    current_dir_len: usize,
}

impl Shell {
    fn new() -> Self {
        let mut current_dir = [0u8; 256];
        current_dir[0] = b'/';
        Self {
            current_dir,
            current_dir_len: 1,
        }
    }

    fn run(&mut self) -> ! {
        let mut line = [0u8; 256];
        
        loop {
            self.print_prompt();
            
            let bytes_read = self.read_line(&mut line);
            
            if bytes_read == 0 {
                continue;
            }
            
            let input = &line[..bytes_read];
            
            if input == b"exit" || input == b"quit" {
                println("Goodbye!");
                loop {}
            }
            
            self.execute_line(input);
        }
    }

    fn read_line(&self, buf: &mut [u8]) -> usize {
        let args = SyscallArgs::new(SYSCALL_READ)
            .with_3args(0, buf.as_mut_ptr() as u64, buf.len() as u64);
        
        unsafe {
            match syscall_api::syscall(args) {
                Ok(n) => n as usize,
                Err(_) => 0,
            }
        }
    }

    fn execute_line(&mut self, input: &[u8]) {
        let trimmed = input.trim();
        
        if trimmed.is_empty() {
            return;
        }
        
        let mut parser = Parser::new(trimmed);
        
        match parser.parse() {
            Some((cmd, args)) => self.execute(cmd, args),
            None => println("Parse error"),
        }
    }

    fn execute(&mut self, cmd: &[u8], args: &[&[u8]]) {
        match cmd {
            b"help" => self.cmd_help(),
            b"echo" => self.cmd_echo(args),
            b"pwd" => self.cmd_pwd(),
            b"ls" => self.cmd_ls(),
            b"cd" => self.cmd_cd(args),
            b"cat" => self.cmd_cat(args),
            b"ps" => self.cmd_ps(),
            b"hostname" => self.cmd_hostname(),
            b"date" => self.cmd_date(),
            b"whoami" => self.cmd_whoami(),
            b"clear" => self.cmd_clear(),
            _ => {
                print("Unknown command: ");
                println_bytes(cmd);
            }
        }
    }

    fn cmd_help(&self) {
        println("Available commands:");
        println!("  help     - Show this help message");
        println!("  echo     - Print text");
        println!("  pwd      - Print working directory");
        println!("  ls       - List directory contents");
        println!("  cd       - Change directory");
        println!("  cat      - Display file contents");
        println!("  ps       - List processes");
        println!("  hostname - Show hostname");
        println!("  date     - Show current date/time");
        println!("  whoami   - Show current user");
        println!("  clear    - Clear screen");
        println!("  exit     - Exit shell");
    }

    fn cmd_echo(&self, args: &[&[u8]]) {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                print(" ");
            }
            print_bytes(arg);
        }
        println("");
    }

    fn cmd_pwd(&self) {
        println_bytes(&self.current_dir[..self.current_dir_len]);
        println("");
    }

    fn cmd_ls(&self) {
        println!(".");
        println!("..");
        println!("bin");
        println!("etc");
        println!("home");
        println!("tmp");
        println!("usr");
    }

    fn cmd_cd(&mut self, args: &[&[u8]]) {
        if args.is_empty() {
            self.current_dir[0] = b'/';
            self.current_dir_len = 1;
            return;
        }
        
        let target = args[0];
        
        if target == b".." {
            if self.current_dir_len > 1 {
                for i in (0..self.current_dir_len).rev() {
                    if self.current_dir[i] == b'/' {
                        self.current_dir_len = i;
                        break;
                    }
                }
                if self.current_dir_len == 0 {
                    self.current_dir[0] = b'/';
                    self.current_dir_len = 1;
                }
            }
        } else if target.starts_with(b"/") {
            let len = target.len().min(255);
            self.current_dir[..len].copy_from_slice(&target[..len]);
            self.current_dir_len = len;
        } else {
            if self.current_dir[self.current_dir_len - 1] != b'/' {
                if self.current_dir_len < 255 {
                    self.current_dir[self.current_dir_len] = b'/';
                    self.current_dir_len += 1;
                }
            }
            let len = target.len().min(255 - self.current_dir_len);
            self.current_dir[self.current_dir_len..self.current_dir_len + len]
                .copy_from_slice(&target[..len]);
            self.current_dir_len += len;
        }
    }

    fn cmd_cat(&self, args: &[&[u8]]) {
        if args.is_empty() {
            println!("Usage: cat <file>");
            return;
        }
        
        let file = args[0];
        
        if file == b"/welcome.txt" || file == b"welcome.txt" {
            println!("Welcome to SIGRUN!");
            println!("This is an immutable filesystem.");
        } else {
            print("cat: ");
            println_bytes(file);
            println(": No such file or directory");
        }
    }

    fn cmd_ps(&self) {
        println!("  PID TTY          TIME CMD");
        println!("    1 ?        00:00:00 init");
        println!("    2 ?        00:00:00 driver-manager");
        println!("    3 ?        00:00:00 filesystem");
        println!("    4 ?        00:00:00 network");
        println!("  100 ?        00:00:00 shell");
    }

    fn cmd_hostname(&self) {
        println!("sigrun");
    }

    fn cmd_date(&self) {
        println!("Thu Feb 19 00:00:00 UTC 2026");
    }

    fn cmd_whoami(&self) {
        println!("root");
    }

    fn cmd_clear(&self) {
        for _ in 0..40 {
            println("");
        }
    }

    fn print_prompt(&self) {
        print_bytes(&self.current_dir[..self.current_dir_len]);
        print("$ ");
    }
}

fn print(s: &str) {
    let bytes = s.as_bytes();
    let args = SyscallArgs::new(SYSCALL_WRITE)
        .with_3args(1, bytes.as_ptr() as u64, bytes.len() as u64);
    unsafe {
        syscall_api::syscall(args).ok();
    }
}

fn print_bytes(bytes: &[u8]) {
    let args = SyscallArgs::new(SYSCALL_WRITE)
        .with_3args(1, bytes.as_ptr() as u64, bytes.len() as u64);
    unsafe {
        syscall_api::syscall(args).ok();
    }
}

fn println(s: &str) {
    print(s);
    print("\n");
}
