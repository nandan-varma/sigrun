//! IPC Message types and structures
//!
//! Defines the message format for inter-process communication,
//! including inline payloads and capability transfer.

use core::sync::atomic::{AtomicU64, Ordering};

pub const MAX_INLINE_CAPS: usize = 4;
pub const MAX_INLINE_PAYLOAD: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MessageType {
    Call = 0,
    Send = 1,
    Recv = 2,
    Signal = 3,
    ShareMemory = 4,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MessageFlags: u8 {
        const NONE = 0;
        const BLOCKING = 1 << 0;
        const PRIORITY = 1 << 1;
        const CAP_TRANSFER = 1 << 2;
        const SHMEM_TRANSFER = 1 << 3;
        const REPLY_EXPECTED = 1 << 4;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MessageHeader {
    pub size: u32,
    pub cap_count: u8,
    pub msg_type: MessageType,
    pub sender_pid: u64,
    pub priority: u8,
    pub flags: MessageFlags,
    pub reserved: [u8; 5],
}

impl MessageHeader {
    pub fn new(msg_type: MessageType, sender_pid: u64) -> Self {
        Self {
            size: 0,
            cap_count: 0,
            msg_type,
            sender_pid,
            priority: 128,
            flags: MessageFlags::NONE,
            reserved: [0; 5],
        }
    }

    pub fn with_size(mut self, size: u32) -> Self {
        self.size = size;
        self
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_flags(mut self, flags: MessageFlags) -> Self {
        self.flags = flags;
        self
    }

    pub fn with_caps(mut self, count: u8) -> Self {
        self.cap_count = count.min(MAX_INLINE_CAPS as u8);
        self
    }
}

#[derive(Debug, Clone)]
pub struct MessageId(u64);

impl MessageId {
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: MessageId,
    pub header: MessageHeader,
    pub inline_caps: [Option<u64>; MAX_INLINE_CAPS],
    pub inline_payload: [u8; MAX_INLINE_PAYLOAD],
    pub payload_len: usize,
    pub extra_caps_handle: u32,
}

impl Message {
    pub fn new(msg_type: MessageType, sender_pid: u64) -> Self {
        Self {
            id: MessageId::new(),
            header: MessageHeader::new(msg_type, sender_pid),
            inline_caps: [None; MAX_INLINE_CAPS],
            inline_payload: [0; MAX_INLINE_PAYLOAD],
            payload_len: 0,
            extra_caps_handle: 0,
        }
    }

    pub fn call(sender_pid: u64) -> Self {
        let mut msg = Self::new(MessageType::Call, sender_pid);
        msg.header.flags |= MessageFlags::REPLY_EXPECTED | MessageFlags::BLOCKING;
        msg
    }

    pub fn send(sender_pid: u64) -> Self {
        Self::new(MessageType::Send, sender_pid)
    }

    pub fn signal(sender_pid: u64) -> Self {
        Self::new(MessageType::Signal, sender_pid)
    }

    pub fn share_memory(sender_pid: u64) -> Self {
        let mut msg = Self::new(MessageType::ShareMemory, sender_pid);
        msg.header.flags |= MessageFlags::SHMEM_TRANSFER;
        msg
    }

    pub fn with_payload(mut self, data: &[u8]) -> Self {
        let len = data.len().min(MAX_INLINE_PAYLOAD);
        self.inline_payload[..len].copy_from_slice(&data[..len]);
        self.payload_len = len;
        self.header.size = len as u32;
        self
    }

    pub fn with_cap(mut self, cap_id: u64) -> Self {
        for i in 0..MAX_INLINE_CAPS {
            if self.inline_caps[i].is_none() {
                self.inline_caps[i] = Some(cap_id);
                self.header.cap_count += 1;
                self.header.flags |= MessageFlags::CAP_TRANSFER;
                break;
            }
        }
        self
    }

    pub fn with_caps(mut self, caps: &[u64]) -> Self {
        for &cap in caps.iter().take(MAX_INLINE_CAPS) {
            self = self.with_cap(cap);
        }
        self
    }

    pub fn payload(&self) -> &[u8] {
        &self.inline_payload[..self.payload_len]
    }

    pub fn caps(&self) -> impl Iterator<Item = u64> + '_ {
        self.inline_caps.iter().filter_map(|c| *c)
    }

    pub fn is_blocking(&self) -> bool {
        self.header.flags.contains(MessageFlags::BLOCKING)
    }

    pub fn expects_reply(&self) -> bool {
        self.header.flags.contains(MessageFlags::REPLY_EXPECTED)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Deadline {
    pub ticks: u64,
}

impl Deadline {
    pub const fn from_ticks(ticks: u64) -> Self {
        Self { ticks }
    }

    pub const fn never() -> Self {
        Self { ticks: u64::MAX }
    }

    pub const fn immediate() -> Self {
        Self { ticks: 0 }
    }

    pub fn is_expired(&self, current_ticks: u64) -> bool {
        current_ticks >= self.ticks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_creation() {
        let msg = Message::call(1);
        assert_eq!(msg.header.msg_type, MessageType::Call);
        assert!(msg.expects_reply());
        assert!(msg.is_blocking());
    }

    #[test]
    fn test_message_payload() {
        let data = [1u8, 2, 3, 4, 5];
        let msg = Message::send(1).with_payload(&data);
        assert_eq!(msg.payload_len, 5);
        assert_eq!(msg.payload(), &data);
    }

    #[test]
    fn test_message_caps() {
        let msg = Message::send(1).with_cap(42).with_cap(100);
        assert_eq!(msg.header.cap_count, 2);
        let caps: Vec<u64> = msg.caps().collect();
        assert_eq!(caps, vec![42, 100]);
    }

    #[test]
    fn test_deadline() {
        let d = Deadline::never();
        assert!(!d.is_expired(u64::MAX - 1));

        let d = Deadline::immediate();
        assert!(d.is_expired(0));
    }
}
