//! Capability Transfer - Moving capabilities between processes
//!
//! Implements secure capability transfer during IPC operations.
//! Capabilities can be moved, copied, or loaned between processes.

use super::table::CapabilityTable;
use super::{CapError, Capability, CapabilityId, SlotId};
use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use spin::Mutex;

extern crate alloc;

/// Global capability tables registry
///
/// Maps process IDs to their capability tables
pub struct CapabilityRegistry {
    tables: BTreeMap<u64, Arc<Mutex<CapabilityTable>>>,
}

impl CapabilityRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tables: BTreeMap::new(),
        }
    }

    /// Register a new process with an empty capability table
    pub fn register_process(&mut self, process_id: u64) -> Arc<Mutex<CapabilityTable>> {
        let table = Arc::new(Mutex::new(CapabilityTable::new(process_id)));
        self.tables.insert(process_id, table.clone());
        table
    }

    /// Get a process's capability table
    pub fn get_table(&self, process_id: u64) -> Option<Arc<Mutex<CapabilityTable>>> {
        self.tables.get(&process_id).cloned()
    }

    /// Remove a process's capability table (on process exit)
    pub fn unregister_process(&mut self, process_id: u64) {
        self.tables.remove(&process_id);
    }

    /// Get number of registered processes
    pub fn process_count(&self) -> usize {
        self.tables.len()
    }
}

impl Default for CapabilityRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Transfer mode for capability passing
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    /// Move capability - source loses it, target gains it
    Move,
    /// Copy capability - both have independent capabilities
    Copy,
    /// Loan capability - temporary access, revocable
    Loan,
}

/// Result of a capability transfer
#[derive(Debug)]
pub struct TransferResult {
    /// Slot in target's table where capability was placed
    pub target_slot: SlotId,
    /// Whether source still has the capability
    pub source_retains: bool,
}

/// Transfer a capability between processes
///
/// # Arguments
/// * `registry` - Global capability registry
/// * `source_pid` - Source process ID
/// * `source_slot` - Slot in source's capability table
/// * `target_pid` - Target process ID
/// * `mode` - How to transfer the capability
///
/// # Returns
/// The slot in the target's table where the capability was placed
pub fn transfer_capability(
    registry: &CapabilityRegistry,
    source_pid: u64,
    source_slot: SlotId,
    target_pid: u64,
    mode: TransferMode,
) -> Result<TransferResult, CapError> {
    if source_pid == target_pid {
        return Err(CapError::SameProcess);
    }

    let source_table = registry
        .get_table(source_pid)
        .ok_or(CapError::ProcessNotFound)?;

    let target_table = registry
        .get_table(target_pid)
        .ok_or(CapError::ProcessNotFound)?;

    match mode {
        TransferMode::Move => {
            let mut source = source_table.lock();
            let cap = source.remove(source_slot)?;

            let mut target = target_table.lock();
            let target_slot = target.insert(cap)?;

            Ok(TransferResult {
                target_slot,
                source_retains: false,
            })
        }
        TransferMode::Copy => {
            let source = source_table.lock();
            let cap = source.lookup(source_slot)?.clone();
            drop(source);

            let mut target = target_table.lock();
            let target_slot = target.insert(cap)?;

            Ok(TransferResult {
                target_slot,
                source_retains: true,
            })
        }
        TransferMode::Loan => {
            let source = source_table.lock();
            let original = source.lookup(source_slot)?;

            let loaned = Capability {
                id: CapabilityId::new(),
                object_type: original.object_type,
                object_id: original.object_id,
                rights: original.rights,
            };
            drop(source);

            let mut target = target_table.lock();
            let target_slot = target.insert(loaned)?;

            Ok(TransferResult {
                target_slot,
                source_retains: true,
            })
        }
    }
}

/// Capability message payload for IPC
///
/// Bundles capabilities with data for IPC messages
#[derive(Debug, Clone)]
pub struct CapabilityMessage {
    /// Message data
    pub data: [u8; 64],
    /// Length of valid data
    pub data_len: usize,
    /// Capabilities to transfer
    pub capabilities: Vec<(SlotId, TransferMode)>,
}

impl CapabilityMessage {
    /// Create a new empty message
    pub fn new() -> Self {
        Self {
            data: [0; 64],
            data_len: 0,
            capabilities: Vec::new(),
        }
    }

    /// Create message from bytes
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut data = [0u8; 64];
        let len = bytes.len().min(64);
        data[..len].copy_from_slice(&bytes[..len]);

        Self {
            data,
            data_len: len,
            capabilities: Vec::new(),
        }
    }

    /// Add a capability to transfer
    pub fn add_capability(&mut self, slot: SlotId, mode: TransferMode) {
        self.capabilities.push((slot, mode));
    }

    /// Get message data as bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data[..self.data_len]
    }
}

impl Default for CapabilityMessage {
    fn default() -> Self {
        Self::new()
    }
}

/// Revoke a loaned capability
///
/// When a capability was loaned, the lender can revoke it,
/// removing it from the borrower's table.
pub fn revoke_capability(
    registry: &CapabilityRegistry,
    owner_pid: u64,
    owner_slot: SlotId,
    borrower_pid: u64,
) -> Result<(), CapError> {
    let owner_table = registry
        .get_table(owner_pid)
        .ok_or(CapError::ProcessNotFound)?;

    let borrower_table = registry
        .get_table(borrower_pid)
        .ok_or(CapError::ProcessNotFound)?;

    let owner = owner_table.lock();
    let owner_cap = owner.lookup(owner_slot)?;
    let object_id = owner_cap.object_id;
    let object_type = owner_cap.object_type;
    drop(owner);

    let mut borrower = borrower_table.lock();

    let slot_to_remove = borrower
        .find_by_object(object_id)
        .find(|(_, cap)| cap.object_type == object_type)
        .map(|(slot, _)| slot);

    if let Some(slot) = slot_to_remove {
        borrower.remove(slot)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capability::{CapRights, Capability, CapabilityId, ObjectType};

    fn make_cap(id: u64) -> Capability {
        Capability {
            id: CapabilityId::new(),
            object_type: ObjectType::Frame,
            object_id: id,
            rights: CapRights::READ | CapRights::WRITE | CapRights::GRANT,
        }
    }

    #[test]
    fn test_registry_register() {
        let mut registry = CapabilityRegistry::new();
        let table = registry.register_process(1);

        assert!(registry.get_table(1).is_some());
        assert_eq!(table.lock().owner(), 1);
    }

    #[test]
    fn test_transfer_move() {
        let mut registry = CapabilityRegistry::new();
        registry.register_process(1);
        registry.register_process(2);

        let table1 = registry.get_table(1).unwrap();
        let cap = make_cap(100);
        let source_slot = table1.lock().insert(cap).unwrap();

        let result = transfer_capability(&registry, 1, source_slot, 2, TransferMode::Move).unwrap();

        assert!(!result.source_retains);
        assert!(table1.lock().lookup(source_slot).is_err());

        let table2 = registry.get_table(2).unwrap();
        let transferred = table2.lock().lookup(result.target_slot).unwrap();
        assert_eq!(transferred.object_id, 100);
    }

    #[test]
    fn test_transfer_copy() {
        let mut registry = CapabilityRegistry::new();
        registry.register_process(1);
        registry.register_process(2);

        let table1 = registry.get_table(1).unwrap();
        let cap = make_cap(200);
        let source_slot = table1.lock().insert(cap).unwrap();

        let result = transfer_capability(&registry, 1, source_slot, 2, TransferMode::Copy).unwrap();

        assert!(result.source_retains);
        assert!(table1.lock().lookup(source_slot).is_some());

        let table2 = registry.get_table(2).unwrap();
        let copied = table2.lock().lookup(result.target_slot).unwrap();
        assert_eq!(copied.object_id, 200);
    }

    #[test]
    fn test_transfer_same_process_fails() {
        let mut registry = CapabilityRegistry::new();
        registry.register_process(1);

        let table1 = registry.get_table(1).unwrap();
        let cap = make_cap(300);
        let slot = table1.lock().insert(cap).unwrap();

        let result = transfer_capability(&registry, 1, slot, 1, TransferMode::Copy);

        assert!(matches!(result, Err(CapError::SameProcess)));
    }

    #[test]
    fn test_capability_message() {
        let mut msg = CapabilityMessage::from_bytes(b"hello");
        msg.add_capability(SlotId::new(0), TransferMode::Copy);

        assert_eq!(msg.as_bytes(), b"hello");
        assert_eq!(msg.capabilities.len(), 1);
    }
}
