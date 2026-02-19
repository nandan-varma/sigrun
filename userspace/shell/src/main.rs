//! Simple Shell
//!
//! Basic command-line interface.

#![no_std]

pub fn main() -> ! {
    println!("SIGRUN Shell v0.1");
    println!("Type 'help' for commands\n");
    
    loop {
        print!("$ ");
        // Would read and process commands
    }
}
