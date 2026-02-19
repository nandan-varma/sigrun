//! Capability Table - Per-process capability storage
//!
//! Each process has a capability table that stores all capabilities
//! it possesses. Capabilities are accessed via slot indices.

use super::{CapError, CapRights, Capability, CapabilityId, ObjectType};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

extern crate alloc;

/// Maximum number of capabilities per process
pub const MAX_CAPABILITIES: usize = 1024;

/// Slot index in a process's capability table
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SlotId(u32);

impl SlotId {
    pub const fn new(id: u32) -> Self {
        Self(id)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl Default for SlotId {
    fn default() -> Self {
        Self(0)
    }
}

/// Per-process capability table
///
/// Stores capabilities indexed by slot IDs. Provides O(log n) lookup
/// and automatic slot allocation.
pub struct CapabilityTable {
    /// Map from slot ID to capability
    slots: BTreeMap<SlotId, Capability>,
    /// Free slot IDs for reuse
    free_slots: Vec<SlotId>,
    /// Next slot ID to allocate
    next_slot: u32,
    /// Owner process ID
    owner: u64,
}

impl CapabilityTable {
    /// Create a new empty capability table
    pub fn new(owner: u64) -> Self {
        Self {
            slots: BTreeMap::new(),
            free_slots: Vec::new(),
            next_slot: 0,
            owner,
        }
    }

    /// Get the owner process ID
    pub fn owner(&self) -> u64 {
        self.owner
    }

    /// Allocate a new slot ID
    fn allocate_slot(&mut self) -> Result<SlotId, CapError> {
        if let Some(slot) = self.free_slots.pop() {
            return Ok(slot);
        }

        if self.next_slot as usize >= MAX_CAPABILITIES {
            return Err(CapError::TableFull);
        }

        let slot = SlotId::new(self.next_slot);
        self.next_slot += 1;
        Ok(slot)
    }

    /// Insert a capability into the table
    ///
    /// Returns the slot ID where the capability was inserted
    pub fn insert(&mut self, cap: Capability) -> Result<SlotId, CapError> {
        let slot = self.allocate_slot()?;
        self.slots.insert(slot, cap);
        Ok(slot)
    }

    /// Look up a capability by slot ID
    pub fn lookup(&self, slot: SlotId) -> Result<&Capability, CapError> {
        self.slots.get(&slot).ok_or(CapError::InvalidSlot)
    }

    /// Look up a capability mutably by slot ID
    pub fn lookup_mut(&mut self, slot: SlotId) -> Result<&mut Capability, CapError> {
        self.slots.get_mut(&slot).ok_or(CapError::InvalidSlot)
    }

    /// Remove a capability from the table
    ///
    /// Returns the removed capability, or error if slot was empty
    pub fn remove(&mut self, slot: SlotId) -> Result<Capability, CapError> {
        let cap = self.slots.remove(&slot).ok_or(CapError::InvalidSlot)?;
        self.free_slots.push(slot);
        Ok(cap)
    }

    /// Check if a slot contains a capability with specific rights
    pub fn check_rights(&self, slot: SlotId, required: CapRights) -> Result<(), CapError> {
        let cap = self.lookup(slot)?;
        if cap.rights.contains(required) {
            Ok(())
        } else {
            Err(CapError::RightsViolation)
        }
    }

    /// Derive a capability with reduced rights
    ///
    /// Creates a new capability from an existing one with a subset of rights.
    /// The new capability references the same underlying object.
    pub fn derive(
        &mut self,
        parent_slot: SlotId,
        new_rights: CapRights,
    ) -> Result<SlotId, CapError> {
        let parent = self.lookup(parent_slot)?;

        if !parent.rights.contains(CapRights::GRANT) {
            return Err(CapError::RightsViolation);
        }

        if !parent.rights.contains(new_rights) {
            return Err(CapError::CannotDeriveMoreRights);
        }

        let derived = Capability {
            id: CapabilityId::new(),
            object_type: parent.object_type,
            object_id: parent.object_id,
            rights: new_rights,
        };

        self.insert(derived)
    }

    /// Get all slot IDs in this table
    pub fn slots(&self) -> impl Iterator<Item = SlotId> + '_ {
        self.slots.keys().copied()
    }

    /// Get number of capabilities in the table
    pub fn len(&self) -> usize {
        self.slots.len()
    }

    /// Check if table is empty
    pub fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Find capabilities by object type
    pub fn find_by_type(
        &self,
        object_type: ObjectType,
    ) -> impl Iterator<Item = (SlotId, &Capability)> {
        self.slots
            .iter()
            .filter(move |(_, cap)| cap.object_type == object_type)
            .map(|(slot, cap)| (*slot, cap))
    }

    /// Find capabilities by object ID
    pub fn find_by_object(&self, object_id: u64) -> impl Iterator<Item = (SlotId, &Capability)> {
        self.slots
            .iter()
            .filter(move |(_, cap)| cap.object_id == object_id)
            .map(|(slot, cap)| (*slot, cap))
    }
}

impl core::fmt::Debug for CapabilityTable {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CapabilityTable")
            .field("owner", &self.owner)
            .field("slot_count", &self.slots.len())
            .field("next_slot", &self.next_slot)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_insert_lookup() {
        let mut table = CapabilityTable::new(1);
        let cap = Capability {
            id: CapabilityId::new(),
            object_type: ObjectType::Process,
            object_id: 42,
            rights: CapRights::READ | CapRights::WRITE,
        };

        let slot = table.insert(cap).unwrap();
        let looked_up = table.lookup(slot).unwrap();

        assert_eq!(looked_up.object_id, 42);
        assert_eq!(looked_up.rights, CapRights::READ | CapRights::WRITE);
    }

    #[test]
    fn test_table_remove() {
        let mut table = CapabilityTable::new(1);
        let cap = Capability {
            id: CapabilityId::new(),
            object_type: ObjectType::Frame,
            object_id: 100,
            rights: CapRights::READ,
        };

        let slot = table.insert(cap).unwrap();
        let removed = table.remove(slot).unwrap();

        assert_eq!(removed.object_id, 100);
        assert!(table.lookup(slot).is_err());
    }

    #[test]
    fn test_slot_reuse() {
        let mut table = CapabilityTable::new(1);

        let cap1 = Capability {
            id: CapabilityId::new(),
            object_type: ObjectType::Process,
            object_id: 1,
            rights: CapRights::READ,
        };
        let slot1 = table.insert(cap1).unwrap();
        table.remove(slot1).unwrap();

        let cap2 = Capability {
            id: CapabilityId::new(),
            object_type: ObjectType::Process,
            object_id: 2,
            rights: CapRights::WRITE,
        };
        let slot2 = table.insert(cap2).unwrap();

        assert_eq!(slot1, slot2);
    }

    #[test]
    fn test_check_rights() {
        let mut table = CapabilityTable::new(1);
        let cap = Capability {
            id: CapabilityId::new(),
            object_type: ObjectType::Process,
            object_id: 1,
            rights: CapRights::READ | CapRights::WRITE,
        };

        let slot = table.insert(cap).unwrap();

        assert!(table.check_rights(slot, CapRights::READ).is_ok());
        assert!(table.check_rights(slot, CapRights::WRITE).is_ok());
        assert!(table
            .check_rights(slot, CapRights::READ | CapRights::WRITE)
            .is_ok());
        assert!(table.check_rights(slot, CapRights::EXECUTE).is_err());
    }

    #[test]
    fn test_derive_capability() {
        let mut table = CapabilityTable::new(1);
        let parent = Capability {
            id: CapabilityId::new(),
            object_type: ObjectType::Process,
            object_id: 1,
            rights: CapRights::READ | CapRights::WRITE | CapRights::GRANT,
        };

        let parent_slot = table.insert(parent).unwrap();

        let child_slot = table.derive(parent_slot, CapRights::READ).unwrap();

        let child = table.lookup(child_slot).unwrap();
        assert_eq!(child.rights, CapRights::READ);
        assert_eq!(child.object_id, 1);
    }

    #[test]
    fn test_derive_requires_grant() {
        let mut table = CapabilityTable::new(1);
        let parent = Capability {
            id: CapabilityId::new(),
            object_type: ObjectType::Process,
            object_id: 1,
            rights: CapRights::READ | CapRights::WRITE,
        };

        let parent_slot = table.insert(parent).unwrap();

        let result = table.derive(parent_slot, CapRights::READ);
        assert!(matches!(result, Err(CapError::RightsViolation)));
    }
}
