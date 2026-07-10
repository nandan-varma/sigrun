//! Async Notification System
//!
//! Provides event multiplexing and async notifications for IPC.
//! Allows processes to wait for multiple events simultaneously.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

use super::endpoint::ProcessId;
use super::message::Deadline;

static NOTIFICATION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct NotificationId(pub u64);

impl NotificationId {
    pub fn new() -> Self {
        Self(NOTIFICATION_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NotificationBits: u64 {
        const BIT_0 = 1 << 0;
        const BIT_1 = 1 << 1;
        const BIT_2 = 1 << 2;
        const BIT_3 = 1 << 3;
        const BIT_4 = 1 << 4;
        const BIT_5 = 1 << 5;
        const BIT_6 = 1 << 6;
        const BIT_7 = 1 << 7;
        const ALL = u64::MAX;
    }
}

#[derive(Debug)]
pub enum WaitError {
    Timeout,
    Cancelled,
    InvalidNotification,
}

pub struct Notification {
    pub id: NotificationId,
    pub process: ProcessId,
    pub slots: [AtomicU64; 4],
}

impl Notification {
    pub fn new(process: ProcessId) -> Self {
        Self {
            id: NotificationId::new(),
            process,
            slots: [
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
                AtomicU64::new(0),
            ],
        }
    }

    pub fn signal(&self, bits: u64) {
        for (i, slot) in self.slots.iter().enumerate() {
            let offset = i * 64;
            let slot_bits = bits.rotate_right(offset as u32) & 0xFFFFFFFFFFFFFFFF;
            slot.fetch_or(slot_bits, Ordering::Release);
        }
    }

    pub fn signal_bit(&self, bit: u8) {
        let slot_idx = (bit / 64) as usize;
        let bit_idx = bit % 64;
        if slot_idx < 4 {
            self.slots[slot_idx].fetch_or(1 << bit_idx, Ordering::Release);
        }
    }

    pub fn clear(&self, mask: u64) {
        for (i, slot) in self.slots.iter().enumerate() {
            let offset = i * 64;
            let slot_bits = mask.rotate_right(offset as u32) & 0xFFFFFFFFFFFFFFFF;
            slot.fetch_and(!slot_bits, Ordering::Release);
        }
    }

    pub fn clear_bit(&self, bit: u8) {
        let slot_idx = (bit / 64) as usize;
        let bit_idx = bit % 64;
        if slot_idx < 4 {
            self.slots[slot_idx].fetch_and(!(1 << bit_idx), Ordering::Release);
        }
    }

    pub fn wait(&self, mask: u64) -> u64 {
        loop {
            let mut signaled = 0u64;

            for (i, slot) in self.slots.iter().enumerate() {
                let offset = i * 64;
                let slot_mask = mask.rotate_right(offset as u32) & 0xFFFFFFFFFFFFFFFF;
                let slot_value = slot.load(Ordering::Acquire);
                signaled |= slot_value.rotate_left(offset as u32) & mask;
            }

            if signaled != 0 {
                self.clear(signaled);
                return signaled;
            }

            core::hint::spin_loop();
        }
    }

    pub fn try_wait(&self, mask: u64) -> Option<u64> {
        let mut signaled = 0u64;

        for (i, slot) in self.slots.iter().enumerate() {
            let offset = i * 64;
            let slot_mask = mask.rotate_right(offset as u32) & 0xFFFFFFFFFFFFFFFF;
            let slot_value = slot.load(Ordering::Acquire);
            signaled |= slot_value.rotate_left(offset as u32) & mask;
        }

        if signaled != 0 {
            self.clear(signaled);
            Some(signaled)
        } else {
            None
        }
    }

    pub fn poll(&self, mask: u64) -> u64 {
        let mut signaled = 0u64;

        for (i, slot) in self.slots.iter().enumerate() {
            let offset = i * 64;
            let slot_mask = mask.rotate_right(offset as u32) & 0xFFFFFFFFFFFFFFFF;
            let slot_value = slot.load(Ordering::Acquire);
            signaled |= slot_value.rotate_left(offset as u32) & mask;
        }

        signaled
    }

    pub fn is_signaled(&self, bit: u8) -> bool {
        let slot_idx = (bit / 64) as usize;
        let bit_idx = bit % 64;
        if slot_idx < 4 {
            let slot_value = self.slots[slot_idx].load(Ordering::Acquire);
            (slot_value & (1 << bit_idx)) != 0
        } else {
            false
        }
    }

    pub fn any_signaled(&self) -> bool {
        self.slots
            .iter()
            .any(|slot| slot.load(Ordering::Acquire) != 0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NotificationBinding {
    pub notification_id: NotificationId,
    pub bit: u8,
}

pub struct NotificationManager {
    notifications: spin::RwLock<BTreeMap<NotificationId, Arc<Notification>>>,
}

impl NotificationManager {
    pub const fn new() -> Self {
        Self {
            notifications: spin::RwLock::new(BTreeMap::new()),
        }
    }

    pub fn create_notification(&self, process: ProcessId) -> Arc<Notification> {
        let notification = Arc::new(Notification::new(process));
        let id = notification.id;
        self.notifications.write().insert(id, notification.clone());
        notification
    }

    pub fn get_notification(&self, id: NotificationId) -> Option<Arc<Notification>> {
        self.notifications.read().get(&id).cloned()
    }

    pub fn destroy_notification(&self, id: NotificationId) -> Result<(), WaitError> {
        self.notifications.write().remove(&id);
        Ok(())
    }

    pub fn signal(&self, id: NotificationId, bits: u64) -> Result<(), WaitError> {
        if let Some(notification) = self.notifications.read().get(&id) {
            notification.signal(bits);
            Ok(())
        } else {
            Err(WaitError::InvalidNotification)
        }
    }

    pub fn signal_bit(&self, id: NotificationId, bit: u8) -> Result<(), WaitError> {
        if let Some(notification) = self.notifications.read().get(&id) {
            notification.signal_bit(bit);
            Ok(())
        } else {
            Err(WaitError::InvalidNotification)
        }
    }

    pub fn notification_count(&self) -> usize {
        self.notifications.read().len()
    }
}

#[derive(Debug)]
pub struct WaitSet {
    pub notifications: Vec<NotificationBinding>,
}

impl WaitSet {
    pub fn new() -> Self {
        Self {
            notifications: Vec::new(),
        }
    }

    pub fn add(&mut self, notification_id: NotificationId, bit: u8) {
        self.notifications.push(NotificationBinding {
            notification_id,
            bit,
        });
    }

    pub fn remove(&mut self, notification_id: NotificationId) {
        self.notifications
            .retain(|n| n.notification_id != notification_id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notification_signal() {
        let process = ProcessId::new();
        let notification = Notification::new(process);

        notification.signal(NotificationBits::BIT_0.bits() | NotificationBits::BIT_1.bits());

        assert!(notification.is_signaled(0));
        assert!(notification.is_signaled(1));
        assert!(!notification.is_signaled(2));
    }

    #[test]
    fn test_notification_wait() {
        let process = ProcessId::new();
        let notification = Notification::new(process);

        notification.signal(NotificationBits::BIT_0.bits());

        let result = notification.try_wait(NotificationBits::BIT_0.bits());
        assert_eq!(result, Some(NotificationBits::BIT_0.bits()));
    }

    #[test]
    fn test_notification_clear() {
        let process = ProcessId::new();
        let notification = Notification::new(process);

        notification.signal(NotificationBits::BIT_0.bits() | NotificationBits::BIT_1.bits());
        notification.clear(NotificationBits::BIT_0.bits());

        assert!(!notification.is_signaled(0));
        assert!(notification.is_signaled(1));
    }

    #[test]
    fn test_notification_manager() {
        let manager = NotificationManager::new();
        let process = ProcessId::new();

        let notification = manager.create_notification(process);
        assert_eq!(manager.notification_count(), 1);

        manager
            .signal(notification.id, NotificationBits::BIT_0.bits())
            .unwrap();
        assert!(notification.is_signaled(0));

        manager.destroy_notification(notification.id).unwrap();
        assert_eq!(manager.notification_count(), 0);
    }
}
