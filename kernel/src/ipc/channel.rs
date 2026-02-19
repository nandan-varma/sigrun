//! IPC Channel management
//!
//! Channels provide bidirectional communication between processes.
//! Each channel has two endpoints that can send and receive messages.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use core::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::endpoint::{Endpoint, EndpointId, EndpointInfo, EndpointRights, ProcessId};
use super::message::{Deadline, Message, MessageType};
use super::queue::{MessageQueue, QueueError, DEFAULT_QUEUE_SIZE};

static CHANNEL_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct ChannelId(u64);

impl ChannelId {
    pub fn new() -> Self {
        Self(CHANNEL_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug)]
pub enum ChannelError {
    CreationFailed,
    InvalidEndpoint,
    QueueFull,
    QueueEmpty,
    QueueClosed,
    PermissionDenied,
    Timeout,
    ProcessNotFound,
}

impl From<QueueError> for ChannelError {
    fn from(e: QueueError) -> Self {
        match e {
            QueueError::Full => ChannelError::QueueFull,
            QueueError::Empty => ChannelError::QueueEmpty,
            QueueError::Closed => ChannelError::QueueClosed,
            QueueError::InvalidCapacity => ChannelError::CreationFailed,
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ChannelFlags: u8 {
        const NONE = 0;
        const SYNCHRONOUS = 1 << 0;
        const PRIORITY_INHERIT = 1 << 1;
        const CAPABILITY_TRANSFER = 1 << 2;
    }
}

pub struct Channel {
    pub id: ChannelId,
    pub endpoint_a: EndpointInfo,
    pub endpoint_b: EndpointInfo,
    queue_a_to_b: Arc<MessageQueue>,
    queue_b_to_a: Arc<MessageQueue>,
    flags: ChannelFlags,
    ref_count: AtomicU64,
}

impl Channel {
    pub fn create(process_a: ProcessId, process_b: ProcessId) -> Result<Self, ChannelError> {
        Self::create_with_capacity(process_a, process_b, DEFAULT_QUEUE_SIZE)
    }

    pub fn create_with_capacity(
        process_a: ProcessId,
        process_b: ProcessId,
        capacity: usize,
    ) -> Result<Self, ChannelError> {
        let id = ChannelId::new();

        let ep_a = Endpoint::new(process_a, super::endpoint::CapabilitySlot::new(0));
        let ep_b = Endpoint::new(process_b, super::endpoint::CapabilitySlot::new(1));

        let info_a = EndpointInfo::new(ep_a, EndpointRights::FULL, id.as_u64());
        let info_b = EndpointInfo::new(ep_b, EndpointRights::FULL, id.as_u64());

        let queue_a_to_b = Arc::new(MessageQueue::new(capacity)?);
        let queue_b_to_a = Arc::new(MessageQueue::new(capacity)?);

        Ok(Self {
            id,
            endpoint_a: info_a,
            endpoint_b: info_b,
            queue_a_to_b,
            queue_b_to_a,
            flags: ChannelFlags::SYNCHRONOUS,
            ref_count: AtomicU64::new(2),
        })
    }

    pub fn create_pair(process: ProcessId) -> Result<(Self, Endpoint, Endpoint), ChannelError> {
        let channel = Self::create(process, process)?;

        let ep_a = channel.endpoint_a.endpoint;
        let ep_b = channel.endpoint_b.endpoint;

        Ok((channel, ep_a, ep_b))
    }

    pub fn send_from_a(&self, msg: Message) -> Result<(), ChannelError> {
        if !self.endpoint_a.can_send() {
            return Err(ChannelError::PermissionDenied);
        }
        self.queue_a_to_b.try_push(msg).map_err(Into::into)
    }

    pub fn send_from_b(&self, msg: Message) -> Result<(), ChannelError> {
        if !self.endpoint_b.can_send() {
            return Err(ChannelError::PermissionDenied);
        }
        self.queue_b_to_a.try_push(msg).map_err(Into::into)
    }

    pub fn recv_at_a(&self) -> Result<Message, ChannelError> {
        if !self.endpoint_a.can_recv() {
            return Err(ChannelError::PermissionDenied);
        }
        self.queue_b_to_a.try_pop().map_err(Into::into)
    }

    pub fn recv_at_b(&self) -> Result<Message, ChannelError> {
        if !self.endpoint_b.can_recv() {
            return Err(ChannelError::PermissionDenied);
        }
        self.queue_a_to_b.try_pop().map_err(Into::into)
    }

    pub fn send(&self, endpoint_id: EndpointId, msg: Message) -> Result<(), ChannelError> {
        if self.endpoint_a.endpoint.id == endpoint_id {
            self.send_from_a(msg)
        } else if self.endpoint_b.endpoint.id == endpoint_id {
            self.send_from_b(msg)
        } else {
            Err(ChannelError::InvalidEndpoint)
        }
    }

    pub fn recv(&self, endpoint_id: EndpointId) -> Result<Message, ChannelError> {
        if self.endpoint_a.endpoint.id == endpoint_id {
            self.recv_at_a()
        } else if self.endpoint_b.endpoint.id == endpoint_id {
            self.recv_at_b()
        } else {
            Err(ChannelError::InvalidEndpoint)
        }
    }

    pub fn call(&self, endpoint_id: EndpointId, mut msg: Message) -> Result<Message, ChannelError> {
        msg.header.msg_type = MessageType::Call;

        self.send(endpoint_id, msg)?;

        let reply = self.recv(endpoint_id)?;

        Ok(reply)
    }

    pub fn reply(&self, endpoint_id: EndpointId, msg: Message) -> Result<(), ChannelError> {
        self.send(endpoint_id, msg)
    }

    pub fn close_endpoint(&mut self, endpoint_id: EndpointId) {
        if self.endpoint_a.endpoint.id == endpoint_id {
            self.endpoint_a.close();
            self.queue_a_to_b.close();
        } else if self.endpoint_b.endpoint.id == endpoint_id {
            self.endpoint_b.close();
            self.queue_b_to_a.close();
        }
    }

    pub fn close(&mut self) {
        self.endpoint_a.close();
        self.endpoint_b.close();
        self.queue_a_to_b.close();
        self.queue_b_to_a.close();
    }

    pub fn is_closed(&self) -> bool {
        self.endpoint_a.is_closed && self.endpoint_b.is_closed
    }

    pub fn pending_at_a(&self) -> usize {
        self.queue_b_to_a.len()
    }

    pub fn pending_at_b(&self) -> usize {
        self.queue_a_to_b.len()
    }

    pub fn id(&self) -> ChannelId {
        self.id
    }

    pub fn flags(&self) -> ChannelFlags {
        self.flags
    }

    pub fn set_flags(&mut self, flags: ChannelFlags) {
        self.flags = flags;
    }
}

pub struct ChannelManager {
    channels: spin::RwLock<BTreeMap<ChannelId, Arc<spin::Mutex<Channel>>>>,
}

impl ChannelManager {
    pub const fn new() -> Self {
        Self {
            channels: spin::RwLock::new(BTreeMap::new()),
        }
    }

    pub fn create_channel(
        &self,
        process_a: ProcessId,
        process_b: ProcessId,
    ) -> Result<ChannelId, ChannelError> {
        let channel = Channel::create(process_a, process_b)?;
        let id = channel.id;
        self.channels
            .write()
            .insert(id, Arc::new(spin::Mutex::new(channel)));
        Ok(id)
    }

    pub fn get_channel(&self, id: ChannelId) -> Option<Arc<spin::Mutex<Channel>>> {
        self.channels.read().get(&id).cloned()
    }

    pub fn destroy_channel(&self, id: ChannelId) -> Result<(), ChannelError> {
        if let Some(channel) = self.channels.write().remove(&id) {
            channel.lock().close();
        }
        Ok(())
    }

    pub fn channel_count(&self) -> usize {
        self.channels.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_creation() {
        let p1 = ProcessId::new();
        let p2 = ProcessId::new();

        let channel = Channel::create(p1, p2).unwrap();
        assert!(!channel.is_closed());
    }

    #[test]
    fn test_channel_send_recv() {
        let p1 = ProcessId::new();
        let p2 = ProcessId::new();

        let channel = Channel::create(p1, p2).unwrap();

        let msg = Message::send(p1.as_u64());
        let ep_a = channel.endpoint_a.endpoint.id;

        channel.send_from_a(msg.clone()).unwrap();

        let received = channel.recv_at_b().unwrap();
        assert_eq!(received.header.sender_pid, p1.as_u64());
    }

    #[test]
    fn test_channel_close() {
        let p1 = ProcessId::new();
        let p2 = ProcessId::new();

        let mut channel = Channel::create(p1, p2).unwrap();
        channel.close();

        assert!(channel.is_closed());
    }
}
