//! Kernel error types

use core::fmt;

/// Generic kernel error
#[derive(Debug)]
pub enum KernelError {
    OutOfMemory,
    InvalidParameter,
    NotFound,
    AlreadyExists,
    PermissionDenied,
    IoError,
    NotSupported,
}

impl fmt::Display for KernelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfMemory => write!(f, "Out of memory"),
            Self::InvalidParameter => write!(f, "Invalid parameter"),
            Self::NotFound => write!(f, "Not found"),
            Self::AlreadyExists => write!(f, "Already exists"),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::IoError => write!(f, "I/O error"),
            Self::NotSupported => write!(f, "Not supported"),
        }
    }
}
