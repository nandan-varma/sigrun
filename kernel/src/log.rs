//! Kernel logging subsystem

use core::fmt::Write;

/// Initialize early logging (before memory manager)
pub fn early_init() {
    // Would set up early console output
    // For now, we just have a no-op that would be replaced with actual serial I/O
}

/// Log a message at INFO level
#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::log::info(&format!($($arg)*));
    };
}

/// Log at INFO level
pub fn info(msg: &str) {
    #[cfg(feature = "debug")]
    {
        // Would output to serial/console
        // For now, we just have a no-op
    }
}

/// Log at ERROR level
pub fn error(msg: &str) {
    // Would output to serial/console
}

/// Debug logging (only in debug builds)
#[cfg(feature = "debug")]
pub fn debug(msg: &str) {
    // Would output to serial/console
}

#[cfg(not(feature = "debug"))]
pub fn debug(_msg: &str) {}
