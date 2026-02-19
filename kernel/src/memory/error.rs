//! Memory-specific error types

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryError {
    OutOfFrames,
    InvalidAddress,
    InvalidSize,
    InvalidAlignment,
    AlreadyMapped,
    NotMapped,
    InvalidFlags,
    RegionOverlap,
    RegionNotFound,
    AddressSpaceFull,
    FrameAllocationFailed,
    PageTableCreationFailed,
    PermissionDenied,
    InvalidOrder,
    BuddySystemCorrupted,
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::OutOfFrames => write!(f, "Out of physical frames"),
            Self::InvalidAddress => write!(f, "Invalid address"),
            Self::InvalidSize => write!(f, "Invalid size"),
            Self::InvalidAlignment => write!(f, "Invalid alignment"),
            Self::AlreadyMapped => write!(f, "Address already mapped"),
            Self::NotMapped => write!(f, "Address not mapped"),
            Self::InvalidFlags => write!(f, "Invalid page flags"),
            Self::RegionOverlap => write!(f, "Memory regions overlap"),
            Self::RegionNotFound => write!(f, "Memory region not found"),
            Self::AddressSpaceFull => write!(f, "Address space full"),
            Self::FrameAllocationFailed => write!(f, "Frame allocation failed"),
            Self::PageTableCreationFailed => write!(f, "Failed to create page table"),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::InvalidOrder => write!(f, "Invalid buddy order"),
            Self::BuddySystemCorrupted => write!(f, "Buddy system corrupted"),
        }
    }
}

impl From<MemoryError> for crate::error::KernelError {
    fn from(err: MemoryError) -> Self {
        match err {
            MemoryError::OutOfFrames
            | MemoryError::FrameAllocationFailed
            | MemoryError::AddressSpaceFull => crate::error::KernelError::OutOfMemory,
            MemoryError::InvalidAddress
            | MemoryError::InvalidSize
            | MemoryError::InvalidAlignment
            | MemoryError::InvalidFlags
            | MemoryError::InvalidOrder => crate::error::KernelError::InvalidParameter,
            MemoryError::PermissionDenied => crate::error::KernelError::PermissionDenied,
            _ => crate::error::KernelError::NotSupported,
        }
    }
}
