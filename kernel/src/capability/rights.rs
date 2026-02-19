//! Rights Management Utilities
//!
//! Helpers for capability rights checking, derivation, and validation.

use super::{CapError, CapRights, Capability, ObjectType};

/// Rights validator for capability operations
pub struct RightsValidator;

impl RightsValidator {
    /// Check if rights are sufficient for read operations
    pub fn can_read(rights: CapRights) -> bool {
        rights.contains(CapRights::READ)
    }

    /// Check if rights are sufficient for write operations
    pub fn can_write(rights: CapRights) -> bool {
        rights.contains(CapRights::WRITE)
    }

    /// Check if rights allow execution
    pub fn can_execute(rights: CapRights) -> bool {
        rights.contains(CapRights::EXECUTE)
    }

    /// Check if rights allow deletion
    pub fn can_delete(rights: CapRights) -> bool {
        rights.contains(CapRights::DELETE)
    }

    /// Check if rights allow granting/deriving
    pub fn can_grant(rights: CapRights) -> bool {
        rights.contains(CapRights::GRANT)
    }

    /// Check if rights allow admin operations
    pub fn is_admin(rights: CapRights) -> bool {
        rights.contains(CapRights::ADMIN)
    }

    /// Validate rights for a specific object type
    pub fn validate_for_type(rights: CapRights, object_type: ObjectType) -> Result<(), CapError> {
        match object_type {
            ObjectType::Process => {
                if rights.contains(CapRights::ADMIN) {
                    Ok(())
                } else if rights.intersects(CapRights::READ | CapRights::WRITE) {
                    Ok(())
                } else {
                    Err(CapError::RightsViolation)
                }
            }
            ObjectType::Thread => {
                if rights.is_empty() {
                    Err(CapError::RightsViolation)
                } else {
                    Ok(())
                }
            }
            ObjectType::AddressSpace => {
                if rights.intersects(CapRights::READ | CapRights::WRITE | CapRights::EXECUTE) {
                    Ok(())
                } else {
                    Err(CapError::RightsViolation)
                }
            }
            ObjectType::Endpoint => {
                if rights.intersects(CapRights::READ | CapRights::WRITE) {
                    Ok(())
                } else {
                    Err(CapError::RightsViolation)
                }
            }
            ObjectType::Frame => {
                if rights.intersects(CapRights::READ | CapRights::WRITE) {
                    Ok(())
                } else {
                    Err(CapError::RightsViolation)
                }
            }
            ObjectType::Device => {
                if rights.intersects(CapRights::READ | CapRights::WRITE | CapRights::ADMIN) {
                    Ok(())
                } else {
                    Err(CapError::RightsViolation)
                }
            }
        }
    }

    /// Check if derivation from parent to child rights is valid
    pub fn can_derive(parent_rights: CapRights, child_rights: CapRights) -> bool {
        if !parent_rights.contains(CapRights::GRANT) {
            return false;
        }
        parent_rights.contains(child_rights)
    }

    /// Get minimum rights for an object type
    pub fn min_rights(object_type: ObjectType) -> CapRights {
        match object_type {
            ObjectType::Process => CapRights::READ,
            ObjectType::Thread => CapRights::READ,
            ObjectType::AddressSpace => CapRights::READ,
            ObjectType::Endpoint => CapRights::READ | CapRights::WRITE,
            ObjectType::Frame => CapRights::READ,
            ObjectType::Device => CapRights::READ,
        }
    }

    /// Get full rights for an object type (for root/admin)
    pub fn full_rights(object_type: ObjectType) -> CapRights {
        match object_type {
            ObjectType::Process => {
                CapRights::READ
                    | CapRights::WRITE
                    | CapRights::DELETE
                    | CapRights::ADMIN
                    | CapRights::GRANT
            }
            ObjectType::Thread => {
                CapRights::READ
                    | CapRights::WRITE
                    | CapRights::DELETE
                    | CapRights::ADMIN
                    | CapRights::GRANT
            }
            ObjectType::AddressSpace => {
                CapRights::READ
                    | CapRights::WRITE
                    | CapRights::EXECUTE
                    | CapRights::DELETE
                    | CapRights::GRANT
            }
            ObjectType::Endpoint => {
                CapRights::READ | CapRights::WRITE | CapRights::DELETE | CapRights::GRANT
            }
            ObjectType::Frame => {
                CapRights::READ
                    | CapRights::WRITE
                    | CapRights::EXECUTE
                    | CapRights::DELETE
                    | CapRights::GRANT
            }
            ObjectType::Device => {
                CapRights::READ
                    | CapRights::WRITE
                    | CapRights::DELETE
                    | CapRights::ADMIN
                    | CapRights::GRANT
            }
        }
    }
}

/// Rights derivation helper
pub struct RightsDerivation;

impl RightsDerivation {
    /// Derive read-only rights from any capability
    pub fn read_only(rights: CapRights) -> CapRights {
        rights & CapRights::READ
    }

    /// Derive write-only rights
    pub fn write_only(rights: CapRights) -> CapRights {
        rights & CapRights::WRITE
    }

    /// Derive read-write rights (no execute, delete, etc.)
    pub fn read_write(rights: CapRights) -> CapRights {
        rights & (CapRights::READ | CapRights::WRITE)
    }

    /// Remove grant right (prevent further delegation)
    pub fn no_delegation(rights: CapRights) -> CapRights {
        rights - CapRights::GRANT
    }

    /// Remove admin right
    pub fn no_admin(rights: CapRights) -> CapRights {
        rights - CapRights::ADMIN
    }

    /// Most restrictive derivation
    pub fn most_restrictive(rights: CapRights) -> CapRights {
        rights & CapRights::READ
    }
}

/// Check if a capability authorizes a specific operation
pub trait AuthorizedOperation {
    /// Required rights for this operation
    fn required_rights() -> CapRights;

    /// Check if capability has required rights
    fn is_authorized(capability: &Capability) -> bool {
        capability.rights.contains(Self::required_rights())
    }
}

/// Read operation marker
pub struct ReadOp;

impl AuthorizedOperation for ReadOp {
    fn required_rights() -> CapRights {
        CapRights::READ
    }
}

/// Write operation marker
pub struct WriteOp;

impl AuthorizedOperation for WriteOp {
    fn required_rights() -> CapRights {
        CapRights::WRITE
    }
}

/// Execute operation marker
pub struct ExecuteOp;

impl AuthorizedOperation for ExecuteOp {
    fn required_rights() -> CapRights {
        CapRights::EXECUTE
    }
}

/// Delete operation marker
pub struct DeleteOp;

impl AuthorizedOperation for DeleteOp {
    fn required_rights() -> CapRights {
        CapRights::DELETE
    }
}

/// Grant operation marker
pub struct GrantOp;

impl AuthorizedOperation for GrantOp {
    fn required_rights() -> CapRights {
        CapRights::GRANT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_read() {
        assert!(RightsValidator::can_read(CapRights::READ));
        assert!(RightsValidator::can_read(
            CapRights::READ | CapRights::WRITE
        ));
        assert!(!RightsValidator::can_read(CapRights::WRITE));
    }

    #[test]
    fn test_can_derive() {
        let parent = CapRights::READ | CapRights::WRITE | CapRights::GRANT;

        assert!(RightsValidator::can_derive(parent, CapRights::READ));
        assert!(RightsValidator::can_derive(
            parent,
            CapRights::READ | CapRights::WRITE
        ));
        assert!(!RightsValidator::can_derive(parent, CapRights::EXECUTE));

        let no_grant = CapRights::READ | CapRights::WRITE;
        assert!(!RightsValidator::can_derive(no_grant, CapRights::READ));
    }

    #[test]
    fn test_derivation_helpers() {
        let full = CapRights::READ | CapRights::WRITE | CapRights::EXECUTE | CapRights::GRANT;

        assert_eq!(RightsDerivation::read_only(full), CapRights::READ);
        assert_eq!(RightsDerivation::write_only(full), CapRights::WRITE);
        assert_eq!(
            RightsDerivation::read_write(full),
            CapRights::READ | CapRights::WRITE
        );
        assert_eq!(
            RightsDerivation::no_delegation(full),
            full - CapRights::GRANT
        );
    }

    #[test]
    fn test_authorized_operations() {
        let cap = Capability {
            id: super::super::CapabilityId::new(),
            object_type: ObjectType::Frame,
            object_id: 1,
            rights: CapRights::READ | CapRights::WRITE,
        };

        assert!(ReadOp::is_authorized(&cap));
        assert!(WriteOp::is_authorized(&cap));
        assert!(!ExecuteOp::is_authorized(&cap));
        assert!(!DeleteOp::is_authorized(&cap));
    }

    #[test]
    fn test_validate_for_type() {
        assert!(RightsValidator::validate_for_type(CapRights::READ, ObjectType::Frame).is_ok());

        assert!(RightsValidator::validate_for_type(CapRights::NONE, ObjectType::Frame).is_err());

        assert!(RightsValidator::validate_for_type(
            CapRights::READ | CapRights::WRITE,
            ObjectType::Endpoint
        )
        .is_ok());
    }
}
