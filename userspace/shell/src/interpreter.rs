//! Command interpreter for the shell

use alloc::vec::Vec;

extern crate alloc;

pub struct Parser {
    input: usize,
    len: usize,
    buffer: usize,
}

impl Parser {
    pub fn new(input: &[u8]) -> Self {
        Self {
            input: 0,
            len: input.len(),
            buffer: input.as_ptr() as usize,
        }
    }

    pub fn parse(&mut self) -> Option<(&[u8], Vec<&[u8]>)> {
        self.skip_whitespace();

        if self.input >= self.len {
            return None;
        }

        let cmd_start = self.input;

        while self.input < self.len {
            let c = unsafe { *((self.buffer + self.input) as *const u8) };
            if c == b' ' || c == b'\t' || c == b'\n' {
                break;
            }
            self.input += 1;
        }

        let cmd = unsafe {
            core::slice::from_raw_parts(
                (self.buffer + cmd_start) as *const u8,
                self.input - cmd_start,
            )
        };

        let mut args = Vec::new();

        while self.input < self.len {
            self.skip_whitespace();

            if self.input >= self.len {
                break;
            }

            let arg_start = self.input;

            while self.input < self.len {
                let c = unsafe { *((self.buffer + self.input) as *const u8) };
                if c == b' ' || c == b'\t' || c == b'\n' {
                    break;
                }
                self.input += 1;
            }

            if self.input > arg_start {
                let arg = unsafe {
                    core::slice::from_raw_parts(
                        (self.buffer + arg_start) as *const u8,
                        self.input - arg_start,
                    )
                };
                args.push(arg);
            }
        }

        Some((cmd, args))
    }

    fn skip_whitespace(&mut self) {
        while self.input < self.len {
            let c = unsafe { *((self.buffer + self.input) as *const u8) };
            if c != b' ' && c != b'\t' && c != b'\n' && c != b'\r' {
                break;
            }
            self.input += 1;
        }
    }
}
