//! Lock-free message queue for IPC
//!
//! Implements a single-producer single-consumer (SPSC) ring buffer
//! for efficient message passing between processes.

extern crate alloc;

use alloc::alloc::{alloc, dealloc};
use core::mem::MaybeUninit;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::message::Message;

pub const DEFAULT_QUEUE_SIZE: usize = 16;

#[derive(Debug)]
pub enum QueueError {
    Full,
    Empty,
    Closed,
    InvalidCapacity,
}

pub struct MessageQueue {
    buffer: *mut MaybeUninit<Message>,
    capacity: usize,
    mask: usize,
    head: AtomicU64,
    tail: AtomicU64,
    is_closed: AtomicBool,
}

unsafe impl Send for MessageQueue {}
unsafe impl Sync for MessageQueue {}

impl MessageQueue {
    pub fn new(capacity: usize) -> Result<Self, QueueError> {
        if capacity == 0 || !capacity.is_power_of_two() {
            return Err(QueueError::InvalidCapacity);
        }

        let layout = core::alloc::Layout::array::<MaybeUninit<Message>>(capacity)
            .map_err(|_| QueueError::InvalidCapacity)?;

        let buffer = unsafe { alloc(layout) as *mut MaybeUninit<Message> };
        if buffer.is_null() {
            return Err(QueueError::InvalidCapacity);
        }

        Ok(Self {
            buffer,
            capacity,
            mask: capacity - 1,
            head: AtomicU64::new(0),
            tail: AtomicU64::new(0),
            is_closed: AtomicBool::new(false),
        })
    }

    pub fn try_push(&self, msg: Message) -> Result<(), QueueError> {
        if self.is_closed.load(Ordering::Acquire) {
            return Err(QueueError::Closed);
        }

        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Acquire);

        if tail - head >= self.capacity as u64 {
            return Err(QueueError::Full);
        }

        let index = (tail & self.mask as u64) as usize;
        unsafe {
            (*self.buffer.add(index)).write(msg);
        }

        self.tail.store(tail + 1, Ordering::Release);
        Ok(())
    }

    pub fn try_pop(&self) -> Result<Message, QueueError> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head == tail {
            if self.is_closed.load(Ordering::Acquire) {
                return Err(QueueError::Closed);
            }
            return Err(QueueError::Empty);
        }

        let index = (head & self.mask as u64) as usize;
        let msg = unsafe { (*self.buffer.add(index)).assume_init_read() };

        self.head.store(head + 1, Ordering::Release);
        Ok(msg)
    }

    pub fn peek(&self) -> Option<&Message> {
        let head = self.head.load(Ordering::Relaxed);
        let tail = self.tail.load(Ordering::Acquire);

        if head == tail {
            return None;
        }

        let index = (head & self.mask as u64) as usize;
        Some(unsafe { &*(*self.buffer.add(index)).as_ptr() })
    }

    pub fn len(&self) -> usize {
        let tail = self.tail.load(Ordering::Relaxed);
        let head = self.head.load(Ordering::Relaxed);
        (tail - head) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn is_full(&self) -> bool {
        self.len() >= self.capacity
    }

    pub fn close(&self) {
        self.is_closed.store(true, Ordering::Release);
    }

    pub fn is_closed(&self) -> bool {
        self.is_closed.load(Ordering::Acquire)
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn remaining(&self) -> usize {
        self.capacity - self.len()
    }
}

impl Drop for MessageQueue {
    fn drop(&mut self) {
        while self.try_pop().is_ok() {}

        let layout = core::alloc::Layout::array::<MaybeUninit<Message>>(self.capacity)
            .unwrap_or_else(|_| core::alloc::Layout::new::<MaybeUninit<Message>>());

        unsafe {
            dealloc(self.buffer as *mut u8, layout);
        }
    }
}

#[derive(Debug)]
pub struct QueueStats {
    pub capacity: usize,
    pub len: usize,
    pub is_closed: bool,
}

impl MessageQueue {
    pub fn stats(&self) -> QueueStats {
        QueueStats {
            capacity: self.capacity,
            len: self.len(),
            is_closed: self.is_closed(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ipc::message::MessageType;

    #[test]
    fn test_queue_basic() {
        let queue = MessageQueue::new(16).unwrap();

        let msg = Message::new(MessageType::Send, 1);
        queue.try_push(msg).unwrap();

        assert_eq!(queue.len(), 1);
        assert!(!queue.is_empty());

        let received = queue.try_pop().unwrap();
        assert_eq!(received.header.msg_type, MessageType::Send);
        assert_eq!(queue.len(), 0);
    }

    #[test]
    fn test_queue_full() {
        let queue = MessageQueue::new(4).unwrap();

        for i in 0..4 {
            let msg = Message::new(MessageType::Send, i as u64);
            queue.try_push(msg).unwrap();
        }

        let msg = Message::new(MessageType::Send, 99);
        assert!(matches!(queue.try_push(msg), Err(QueueError::Full)));
    }

    #[test]
    fn test_queue_close() {
        let queue = MessageQueue::new(4).unwrap();

        let msg = Message::new(MessageType::Send, 1);
        queue.try_push(msg).unwrap();

        queue.close();

        let msg = Message::new(MessageType::Send, 2);
        assert!(matches!(queue.try_push(msg), Err(QueueError::Closed)));

        let _ = queue.try_pop().unwrap();
        assert!(matches!(queue.try_pop(), Err(QueueError::Closed)));
    }
}
