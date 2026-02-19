//! System call error types

/// System call error
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SyscallError(i64);

impl SyscallError {
    /// Create from raw error code
    pub const fn from_raw(code: i64) -> Self {
        Self(code)
    }

    /// Get raw error code
    pub fn raw(self) -> i64 {
        self.0
    }

    /// Get error code as positive number
    pub fn code(self) -> u64 {
        (-self.0) as u64
    }
}

pub const EPERM: i64 = 1;
pub const ENOENT: i64 = 2;
pub const ESRCH: i64 = 3;
pub const EINTR: i64 = 4;
pub const EIO: i64 = 5;
pub const ENXIO: i64 = 6;
pub const ENOMEM: i64 = 12;
pub const EACCES: i64 = 13;
pub const EFAULT: i64 = 14;
pub const EBUSY: i64 = 16;
pub const EEXIST: i64 = 17;
pub const ENODEV: i64 = 19;
pub const EINVAL: i64 = 22;
pub const ENOSPC: i64 = 28;
pub const EROFS: i64 = 30;
pub const EPIPE: i64 = 32;
pub const ENOSYS: i64 = 38;

impl core::fmt::Display for SyscallError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let msg = match self.0 {
            EPERM => "Operation not permitted",
            ENOENT => "No such file or directory",
            ESRCH => "No such process",
            EINTR => "Interrupted system call",
            EIO => "I/O error",
            ENXIO => "No such device or address",
            ENOMEM => "Out of memory",
            EACCES => "Permission denied",
            EFAULT => "Bad address",
            EBUSY => "Device or resource busy",
            EEXIST => "File exists",
            ENODEV => "No such device",
            EINVAL => "Invalid argument",
            ENOSPC => "No space left on device",
            EROFS => "Read-only file system",
            EPIPE => "Broken pipe",
            ENOSYS => "Function not implemented",
            _ => "Unknown error",
        };
        write!(f, "SyscallError({}): {}", self.0, msg)
    }
}
