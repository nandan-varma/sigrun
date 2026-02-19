//! Common error types

#[derive(Debug)]
pub enum Error {
    InvalidParameter,
    NotFound,
    PermissionDenied,
    IoError,
    System(u64),
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidParameter => write!(f, "Invalid parameter"),
            Self::NotFound => write!(f, "Not found"),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::IoError => write!(f, "I/O error"),
            Self::System(code) => write!(f, "System error: {}", code),
        }
    }
}
