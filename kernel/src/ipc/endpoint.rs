//! IPC Endpoint management
//!
//! Endpoints represent the connection points for IPC channels.
//! Each endpoint is associated with a process and a capability slot.

use core::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProcessId(u64);

impl ProcessId {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

impl Default for ProcessId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilitySlot(u32);

impl CapabilitySlot {
    pub const fn new(slot: u32) -> Self {
        Self(slot)
    }

    pub fn as_u32(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EndpointId(u64);

impl EndpointId {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Endpoint {
    pub id: EndpointId,
    pub process: ProcessId,
    pub slot: CapabilitySlot,
}

impl Endpoint {
    pub fn new(process: ProcessId, slot: CapabilitySlot) -> Self {
        Self {
            id: EndpointId::new(),
            process,
            slot,
        }
    }

    pub fn for_process(process: ProcessId) -> Self {
        Self {
            id: EndpointId::new(),
            process,
            slot: CapabilitySlot::new(0),
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct EndpointRights: u8 {
        const NONE = 0;
        const SEND = 1 << 0;
        const RECV = 1 << 1;
        const GRANT = 1 << 2;
        const FULL = Self::SEND.bits() | Self::RECV.bits() | Self::GRANT.bits();
    }
}

#[derive(Debug, Clone)]
pub struct EndpointInfo {
    pub endpoint: Endpoint,
    pub rights: EndpointRights,
    pub channel_id: u64,
    pub is_closed: bool,
}

impl EndpointInfo {
    pub fn new(endpoint: Endpoint, rights: EndpointRights, channel_id: u64) -> Self {
        Self {
            endpoint,
            rights,
            channel_id,
            is_closed: false,
        }
    }

    pub fn can_send(&self) -> bool {
        self.rights.contains(EndpointRights::SEND) && !self.is_closed
    }

    pub fn can_recv(&self) -> bool {
        self.rights.contains(EndpointRights::RECV) && !self.is_closed
    }

    pub fn can_grant(&self) -> bool {
        self.rights.contains(EndpointRights::GRANT) && !self.is_closed
    }

    pub fn close(&mut self) {
        self.is_closed = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_id_generation() {
        let p1 = ProcessId::new();
        let p2 = ProcessId::new();
        assert_ne!(p1, p2);
    }

    #[test]
    fn test_endpoint_creation() {
        let pid = ProcessId::new();
        let ep = Endpoint::for_process(pid);
        assert_eq!(ep.process, pid);
    }

    #[test]
    fn test_endpoint_rights() {
        let pid = ProcessId::new();
        let ep = Endpoint::for_process(pid);
        let info = EndpointInfo::new(ep, EndpointRights::FULL, 1);

        assert!(info.can_send());
        assert!(info.can_recv());
        assert!(info.can_grant());
    }
}
