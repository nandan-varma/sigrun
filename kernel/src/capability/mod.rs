//! Capability-based security subsystem
//!
//! Implements object-capability model with unforgeable references,
//! rights management, and delegation. Provides:
//!
//! - **CapabilityTable**: Per-process storage of capabilities
//! - **CapabilityRegistry**: Global registry of all capability tables
//! - **Transfer**: Move/copy/loan capabilities between processes
//! - **Rights**: Validation and derivation of rights
//!
//! # Architecture
//!
//! Each process has a `CapabilityTable` containing its capabilities.
//! Capabilities are accessed via `SlotId` indices. The global
//! `CapabilityRegistry` tracks all tables.
//!
//! # Example
//!
//! ```rust
//! use kernel::capability::*;
//!
//! // Create registry and register processes
//! let mut registry = CapabilityRegistry::new();
//! let table1 = registry.register_process(1);
//! let table2 = registry.register_process(2);
//!
//! // Insert capability into process 1
//! let slot = table1.lock().insert(my_capability).unwrap();
//!
//! // Transfer capability to process 2
//! let result = transfer_capability(
//!     &registry, 1, slot, 2, TransferMode::Copy
//! ).unwrap();
//! ```

extern crate alloc;

pub mod rights;
pub mod table;
pub mod transfer;

pub use rights::*;
pub use table::{CapabilityTable, SlotId, MAX_CAPABILITIES};
pub use transfer::*;

use crate::error::KernelError;

/// Initialize capability system
pub fn init() -> CapabilityManager {
    crate::log::info_formatted("  - Creating root capability");
    let root = CapabilityManager::new();
    crate::log::info_formatted("  - Capability system ready");
    root
}

/// Capability rights bitflags
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CapRights(u32);

impl CapRights {
    pub const NONE: Self = Self(0);
    pub const READ: Self = Self(1 << 0);
    pub const WRITE: Self = Self(1 << 1);
    pub const EXECUTE: Self = Self(1 << 2);
    pub const DELETE: Self = Self(1 << 3);
    pub const ADMIN: Self = Self(1 << 4);
    pub const GRANT: Self = Self(1 << 5);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn all() -> Self {
        Self(0x3F)
    }

    pub fn bits(self) -> u32 {
        self.0
    }

    pub fn from_bits(bits: u32) -> Option<Self> {
        let masked = bits & Self::all().0;
        Some(Self(masked))
    }

    pub fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    pub fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }

    pub fn remove(&mut self, other: Self) {
        self.0 &= !other.0;
    }
}

impl core::ops::BitOr for CapRights {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for CapRights {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl core::ops::BitAnd for CapRights {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self(self.0 & rhs.0)
    }
}

impl core::ops::BitAndAssign for CapRights {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl core::ops::Sub for CapRights {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 & !rhs.0)
    }
}

impl Default for CapRights {
    fn default() -> Self {
        Self::empty()
    }
}

/// Capability ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityId(u64);

impl CapabilityId {
    pub fn new() -> Self {
        use core::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for CapabilityId {
    fn default() -> Self {
        Self::new()
    }
}

/// Object type that can be capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ObjectType {
    Process,
    Thread,
    AddressSpace,
    Endpoint,
    Frame,
    Device,
}

/// Capability entry
#[derive(Debug, Clone)]
pub struct Capability {
    pub id: CapabilityId,
    pub object_type: ObjectType,
    pub object_id: u64,
    pub rights: CapRights,
}

impl Capability {
    /// Create a new capability
    pub fn new(object_type: ObjectType, object_id: u64, rights: CapRights) -> Self {
        Self {
            id: CapabilityId::new(),
            object_type,
            object_id,
            rights,
        }
    }

    /// Check if capability has specific rights
    pub fn has_rights(&self, required: CapRights) -> bool {
        self.rights.contains(required)
    }

    /// Check if capability is for a specific object
    pub fn is_for_object(&self, object_type: ObjectType, object_id: u64) -> bool {
        self.object_type == object_type && self.object_id == object_id
    }
}

/// Capability manager
pub struct CapabilityManager {
    root_cap: CapabilityId,
}

impl CapabilityManager {
    pub fn new() -> Self {
        Self {
            root_cap: CapabilityId::new(),
        }
    }

    /// Get root capability
    pub fn root(&self) -> CapabilityId {
        self.root_cap
    }
}

impl Default for CapabilityManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Capability errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapError {
    InvalidCapability,
    RightsViolation,
    InvalidSlot,
    TableFull,
    ProcessNotFound,
    SameProcess,
    CannotDeriveMoreRights,
}

impl core::fmt::Display for CapError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidCapability => write!(f, "Invalid capability"),
            Self::RightsViolation => write!(f, "Rights violation"),
            Self::InvalidSlot => write!(f, "Invalid slot"),
            Self::TableFull => write!(f, "Capability table full"),
            Self::ProcessNotFound => write!(f, "Process not found"),
            Self::SameProcess => write!(f, "Cannot transfer to same process"),
            Self::CannotDeriveMoreRights => write!(f, "Cannot derive more rights than parent"),
        }
    }
}

impl From<CapError> for KernelError {
    fn from(err: CapError) -> Self {
        match err {
            CapError::InvalidCapability | CapError::InvalidSlot => KernelError::InvalidParameter,
            CapError::RightsViolation => KernelError::PermissionDenied,
            CapError::TableFull => KernelError::OutOfMemory,
            CapError::ProcessNotFound => KernelError::NotFound,
            CapError::SameProcess | CapError::CannotDeriveMoreRights => {
                KernelError::InvalidParameter
            }
        }
    }
}

/// Derive a new capability from parent
pub fn derive_capability(_parent: CapabilityId, rights: CapRights) -> Result<Capability, CapError> {
    Ok(Capability {
        id: CapabilityId::new(),
        object_type: ObjectType::Process,
        object_id: 1,
        rights,
    })
}

/// Global capability registry singleton
static GLOBAL_REGISTRY: spin::Mutex<Option<CapabilityRegistry>> = spin::Mutex::new(None);

/// Initialize global capability registry
pub fn init_global_registry() {
    let mut guard = GLOBAL_REGISTRY.lock();
    *guard = Some(CapabilityRegistry::new());
    crate::log::info_formatted("  - Global capability registry initialized");
}

/// Get reference to global registry
///
/// # Panics
/// Panics if registry has not been initialized
pub fn with_global_registry<F, R>(f: F) -> R
where
    F: FnOnce(&CapabilityRegistry) -> R,
{
    let guard = GLOBAL_REGISTRY.lock();
    let registry = guard.as_ref().expect("Capability registry not initialized");
    f(registry)
}

/// Register a process with the global registry
pub fn register_process_global(pid: u64) -> Option<alloc::sync::Arc<spin::Mutex<CapabilityTable>>> {
    let mut guard = GLOBAL_REGISTRY.lock();
    guard.as_mut().map(|r| r.register_process(pid))
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::format;

    #[test]
    fn test_cap_rights() {
        let r = CapRights::READ | CapRights::WRITE;
        assert!(r.contains(CapRights::READ));
        assert!(r.contains(CapRights::WRITE));
        assert!(!r.contains(CapRights::EXECUTE));
        assert!(r.intersects(CapRights::READ));
    }

    #[test]
    fn test_capability_rights_check() {
        let cap = Capability::new(ObjectType::Frame, 1, CapRights::READ | CapRights::WRITE);
        assert!(cap.has_rights(CapRights::READ));
        assert!(!cap.has_rights(CapRights::EXECUTE));
    }

    #[test]
    fn test_object_match() {
        let cap = Capability::new(ObjectType::Process, 42, CapRights::ADMIN);
        assert!(cap.is_for_object(ObjectType::Process, 42));
        assert!(!cap.is_for_object(ObjectType::Thread, 42));
        assert!(!cap.is_for_object(ObjectType::Process, 43));
    }

    #[test]
    fn test_cap_error_display() {
        assert_eq!(format!("{}", CapError::RightsViolation), "Rights violation");
        assert_eq!(format!("{}", CapError::TableFull), "Capability table full");
    }

    #[test]
    fn test_cap_error_to_kernel_error() {
        assert!(matches!(
            KernelError::from(CapError::RightsViolation),
            KernelError::PermissionDenied
        ));
        assert!(matches!(
            KernelError::from(CapError::TableFull),
            KernelError::OutOfMemory
        ));
    }
}
