//! Kernel logging – writes to COM1 serial port.
//!
//! Formatted output uses `core::fmt::Write` so no heap allocation is needed.

use core::fmt;

pub struct SerialWriter;

impl fmt::Write for SerialWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        #[cfg(target_arch = "x86_64")]
        crate::arch::x86_64::serial::write(s);
        Ok(())
    }
}

pub fn early_init() {
    #[cfg(target_arch = "x86_64")]
    crate::arch::x86_64::serial::init();
}

pub fn info(msg: &str) {
    #[cfg(target_arch = "x86_64")]
    crate::arch::x86_64::serial::writeln(msg);
}

pub fn error(msg: &str) {
    #[cfg(target_arch = "x86_64")]
    {
        crate::arch::x86_64::serial::write("ERROR: ");
        crate::arch::x86_64::serial::writeln(msg);
    }
}

pub fn warn(msg: &str) {
    #[cfg(target_arch = "x86_64")]
    {
        crate::arch::x86_64::serial::write("WARN:  ");
        crate::arch::x86_64::serial::writeln(msg);
    }
}

pub fn debug(msg: &str) {
    #[cfg(all(target_arch = "x86_64", debug_assertions))]
    {
        crate::arch::x86_64::serial::write("DEBUG: ");
        crate::arch::x86_64::serial::writeln(msg);
    }
    let _ = msg;
}

/// Write a formatted line to the serial console.  Does not allocate.
pub fn fmt(args: fmt::Arguments) {
    use fmt::Write;
    let mut w = SerialWriter;
    let _ = w.write_fmt(args);
    // newline after every formatted message
    #[cfg(target_arch = "x86_64")]
    crate::arch::x86_64::serial::writeln("");
}

// Keep old names for callers that haven't been updated yet.
pub fn info_formatted(msg: &str) {
    info(msg);
}
pub fn warn_formatted(msg: &str) {
    warn(msg);
}
pub fn error_formatted(msg: &str) {
    error(msg);
}
pub fn debug_formatted(msg: &str) {
    debug(msg);
}
