//! Inter-Process Communication subsystem
//!
//! Provides message passing, shared memory, and async notifications.
//! This is the main entry point for the IPC system.

pub mod channel;
pub mod endpoint;
pub mod message;
pub mod notification;
pub mod queue;
pub mod shared_memory;
pub mod syscall;

use core::fmt;

pub use channel::{Channel, ChannelError, ChannelFlags, ChannelId, ChannelManager};
pub use endpoint::{CapabilitySlot, Endpoint, EndpointId, EndpointInfo, EndpointRights, ProcessId};
pub use message::{
    Deadline, Message, MessageFlags, MessageHeader, MessageId, MessageType, MAX_INLINE_CAPS,
    MAX_INLINE_PAYLOAD,
};
pub use notification::{
    Notification, NotificationBits, NotificationId, NotificationManager, WaitError, WaitSet,
};
pub use queue::{MessageQueue, QueueError, QueueStats, DEFAULT_QUEUE_SIZE};
pub use shared_memory::{
    MemoryRights, ShareMode, SharedMemoryManager, SharedMemoryRegion, ShmError, ShmHandle, ShmId,
    ShmMapping,
};
pub use syscall::{
    dispatch_ipc_syscall, get_manager, handle_ipc_call, handle_ipc_create, handle_ipc_destroy,
    handle_ipc_recv, handle_ipc_send, init as init_manager, sys_ipc_call, sys_ipc_create,
    sys_ipc_destroy, sys_ipc_notify, sys_ipc_recv, sys_ipc_send, sys_ipc_wait, IpcManager,
};

/// IPC errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpcError {
    InvalidEndpoint,
    ChannelClosed,
    QueueFull,
    QueueEmpty,
    Timeout,
    PermissionDenied,
    ProcessNotFound,
    CreationFailed,
    OutOfMemory,
}

impl fmt::Display for IpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidEndpoint => write!(f, "Invalid endpoint"),
            Self::ChannelClosed => write!(f, "Channel closed"),
            Self::QueueFull => write!(f, "Queue full"),
            Self::QueueEmpty => write!(f, "Queue empty"),
            Self::Timeout => write!(f, "Timeout"),
            Self::PermissionDenied => write!(f, "Permission denied"),
            Self::ProcessNotFound => write!(f, "Process not found"),
            Self::CreationFailed => write!(f, "Channel creation failed"),
            Self::OutOfMemory => write!(f, "Out of memory"),
        }
    }
}

impl From<ChannelError> for IpcError {
    fn from(e: ChannelError) -> Self {
        match e {
            ChannelError::InvalidEndpoint => Self::InvalidEndpoint,
            ChannelError::QueueClosed => Self::ChannelClosed,
            ChannelError::QueueFull => Self::QueueFull,
            ChannelError::QueueEmpty => Self::QueueEmpty,
            ChannelError::Timeout => Self::Timeout,
            ChannelError::PermissionDenied => Self::PermissionDenied,
            ChannelError::CreationFailed => Self::CreationFailed,
            ChannelError::ProcessNotFound => Self::ProcessNotFound,
        }
    }
}

impl From<QueueError> for IpcError {
    fn from(e: QueueError) -> Self {
        match e {
            QueueError::Full => Self::QueueFull,
            QueueError::Empty => Self::QueueEmpty,
            QueueError::Closed => Self::ChannelClosed,
            QueueError::InvalidCapacity => Self::CreationFailed,
        }
    }
}

impl From<ShmError> for IpcError {
    fn from(e: ShmError) -> Self {
        match e {
            ShmError::RegionNotFound => Self::InvalidEndpoint,
            ShmError::MappingFailed => Self::CreationFailed,
            ShmError::InvalidHandle => Self::InvalidEndpoint,
            ShmError::PermissionDenied => Self::PermissionDenied,
            ShmError::OutOfMemory => Self::OutOfMemory,
        }
    }
}

/// Initialize IPC subsystem
pub fn init() {
    crate::log::info_formatted("  - IPC subsystem initializing...");

    syscall::init();

    crate::log::info_formatted("  - Message types registered");
    crate::log::info_formatted("  - Channel manager initialized");
    crate::log::info_formatted("  - Notification system ready");
    crate::log::info_formatted("  - Shared memory manager ready");
    crate::log::info_formatted("  - IPC syscall handlers registered");
}

/// Create a new IPC channel between two processes
///
/// Returns the channel ID on success
pub fn create_channel(process_a: u64, process_b: u64) -> Result<u64, IpcError> {
    let manager = get_manager();
    let channel_id = manager
        .channels
        .create_channel(
            ProcessId::from_raw(process_a),
            ProcessId::from_raw(process_b),
        )
        .map_err(|e| e)?;
    Ok(channel_id.as_u64())
}

/// Destroy an IPC channel
pub fn destroy_channel(channel_id: u64) -> Result<(), IpcError> {
    let manager = get_manager();
    manager
        .channels
        .destroy_channel(ChannelId(channel_id))
        .map_err(|e| e)?;
    Ok(())
}

/// Send a message through a channel
pub fn send_message(channel_id: u64, endpoint_id: u64, msg: Message) -> Result<(), IpcError> {
    let manager = get_manager();
    manager
        .channels
        .get_channel(ChannelId(channel_id))
        .ok_or(IpcError::InvalidEndpoint)?
        .lock()
        .send(EndpointId(endpoint_id), msg)
        .map_err(|_| IpcError::ChannelClosed)
}

/// Receive a message from a channel
pub fn recv_message(channel_id: u64, endpoint_id: u64) -> Result<Message, IpcError> {
    let manager = get_manager();
    manager
        .channels
        .get_channel(ChannelId(channel_id))
        .ok_or(IpcError::InvalidEndpoint)?
        .lock()
        .recv(EndpointId(endpoint_id))
        .map_err(|_| IpcError::ChannelClosed)
}

/// Perform an RPC-style call through a channel
pub fn call(channel_id: u64, endpoint_id: u64, request: Message) -> Result<Message, IpcError> {
    let manager = get_manager();
    manager
        .channels
        .get_channel(ChannelId(channel_id))
        .ok_or(IpcError::InvalidEndpoint)?
        .lock()
        .call(EndpointId(endpoint_id), request)
        .map_err(|_| IpcError::Timeout)
}

/// Create a notification object for async events
pub fn create_notification(process_id: u64) -> u64 {
    let manager = get_manager();
    let notification = manager
        .notifications
        .create_notification(ProcessId::from_raw(process_id));
    notification.id.as_u64()
}

/// Signal a notification with specific bits
pub fn signal_notification(notification_id: u64, bits: u64) -> Result<(), IpcError> {
    let manager = get_manager();
    manager
        .notifications
        .signal(NotificationId(notification_id), bits)
        .map_err(|_| IpcError::InvalidEndpoint)
}

/// Wait for notification events
pub fn wait_notification(notification_id: u64, mask: u64) -> u64 {
    let manager = get_manager();
    if let Some(notification) = manager
        .notifications
        .get_notification(NotificationId(notification_id))
    {
        notification.wait(mask)
    } else {
        0
    }
}

/// Create a shared memory region
pub fn create_shared_memory(owner_id: u64, page_count: usize) -> Result<u64, IpcError> {
    let manager = get_manager();
    let region = manager
        .shared_memory
        .create_region(
            page_count,
            MemoryRights::READ | MemoryRights::WRITE,
            ShareMode::ReadWrite,
            ProcessId::from_raw(owner_id),
        )
        .map_err(|_| IpcError::OutOfMemory)?;
    Ok(region.id.as_u64())
}

/// Destroy a shared memory region
pub fn destroy_shared_memory(region_id: u64) -> Result<(), IpcError> {
    let manager = get_manager();
    manager
        .shared_memory
        .destroy_region(ShmId(region_id))
        .map_err(|_| IpcError::InvalidEndpoint)
}

/// Get IPC statistics
pub fn get_stats() -> IpcStats {
    let manager = get_manager();
    IpcStats {
        channels: manager.channels.channel_count(),
        notifications: manager.notifications.notification_count(),
        shared_memory_regions: manager.shared_memory.region_count(),
    }
}

/// IPC system statistics
#[derive(Debug, Clone, Copy)]
pub struct IpcStats {
    pub channels: usize,
    pub notifications: usize,
    pub shared_memory_regions: usize,
}

impl fmt::Display for IpcStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "IPC Stats: {} channels, {} notifications, {} shared memory regions",
            self.channels, self.notifications, self.shared_memory_regions
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_test() {
        if syscall::IPC_MANAGER.get().is_none() {
            init();
        }
    }

    #[test]
    fn test_ipc_init() {
        init_test();
        let stats = get_stats();
        assert_eq!(stats.channels, 0);
    }

    #[test]
    fn test_channel_lifecycle() {
        init_test();
        let pid1 = 1u64;
        let pid2 = 2u64;

        let channel_id = create_channel(pid1, pid2).unwrap();
        assert!(channel_id > 0);
        assert_eq!(get_stats().channels, 1);

        destroy_channel(channel_id).unwrap();
        assert_eq!(get_stats().channels, 0);
    }

    #[test]
    fn test_notification_lifecycle() {
        init_test();
        let pid = 1u64;

        let notif_id = create_notification(pid);
        assert!(notif_id > 0);

        signal_notification(notif_id, NotificationBits::BIT_0.bits()).unwrap();
    }
}
