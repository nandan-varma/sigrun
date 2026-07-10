//! Kernel logging subsystem - writes to COM1 serial port

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
    crate::arch::x86_64::serial::writeln(msg);
}

pub fn warn(msg: &str) {
    #[cfg(target_arch = "x86_64")]
    crate::arch::x86_64::serial::writeln(msg);
}

pub fn debug(msg: &str) {
    #[cfg(all(target_arch = "x86_64", debug_assertions))]
    crate::arch::x86_64::serial::writeln(msg);
}

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
