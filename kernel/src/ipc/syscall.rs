//! IPC Syscall Interface
//!
//! Provides system calls for inter-process communication:
//! - ipc_create: Create IPC channel
//! - ipc_destroy: Destroy IPC channel
//! - ipc_send: Send message
//! - ipc_recv: Receive message
//! - ipc_call: RPC-style call
//! - ipc_notify: Async notification

use syscall_api::{
    error::{EINVAL, ENOSYS, EPERM},
    SyscallArgs, SyscallError, SyscallResult, SYSCALL_IPC_CREATE, SYSCALL_IPC_DESTROY,
    SYSCALL_IPC_RECV, SYSCALL_IPC_SEND,
};

use super::{
    channel::{Channel, ChannelId, ChannelManager},
    endpoint::{Endpoint, EndpointId, ProcessId},
    message::{Deadline, Message, MessageFlags},
    notification::{Notification, NotificationBits, NotificationId, NotificationManager},
    shared_memory::{MemoryRights, ShareMode, SharedMemoryManager, ShmId},
};

extern crate alloc;

use alloc::sync::Arc;
use spin::Mutex;

pub static IPC_MANAGER: spin::Once<Arc<IpcManager>> = spin::Once::new();

pub struct IpcManager {
    pub channels: ChannelManager,
    pub notifications: NotificationManager,
    pub shared_memory: SharedMemoryManager,
}

impl IpcManager {
    pub const fn new() -> Self {
        Self {
            channels: ChannelManager::new(),
            notifications: NotificationManager::new(),
            shared_memory: SharedMemoryManager::new(),
        }
    }

    pub fn init() -> Arc<Self> {
        Arc::new(Self::new())
    }
}

pub fn get_manager() -> &'static IpcManager {
    IPC_MANAGER.get().expect("IPC manager not initialized")
}

pub fn init() {
    IPC_MANAGER.call_once(IpcManager::init);
    log::info!("  - IPC subsystem initialized with syscall support");
}

pub fn handle_ipc_create(args: &SyscallArgs, caller_pid: u64) -> SyscallResult {
    let target_pid = args.arg0;

    if target_pid == 0 {
        let channel_id = get_manager()
            .channels
            .create_channel(
                ProcessId::from_raw(caller_pid),
                ProcessId::from_raw(caller_pid),
            )
            .map_err(|_| SyscallError::from_raw(EINVAL))?;

        Ok(channel_id.as_u64())
    } else {
        let channel_id = get_manager()
            .channels
            .create_channel(
                ProcessId::from_raw(caller_pid),
                ProcessId::from_raw(target_pid),
            )
            .map_err(|_| SyscallError::from_raw(EINVAL))?;

        Ok(channel_id.as_u64())
    }
}

pub fn handle_ipc_destroy(args: &SyscallArgs) -> SyscallResult {
    let channel_id = args.arg0;

    get_manager()
        .channels
        .destroy_channel(ChannelId(channel_id))
        .map_err(|_| SyscallError::from_raw(EINVAL))?;

    Ok(0)
}

pub fn handle_ipc_send(args: &SyscallArgs, caller_pid: u64) -> SyscallResult {
    let channel_id = args.arg0;
    let payload_ptr = args.arg1;
    let payload_len = args.arg2;
    let flags = args.arg3 as u8;

    let channel = get_manager()
        .channels
        .get_channel(ChannelId(channel_id))
        .ok_or_else(|| SyscallError::from_raw(EINVAL))?;

    let msg = Message::new(super::message::MessageType::Send, caller_pid)
        .with_flags(MessageFlags::from_bits_truncate(flags));

    channel
        .lock()
        .send(EndpointId(args.arg4), msg)
        .map_err(|_| SyscallError::from_raw(EPERM))?;

    Ok(0)
}

pub fn handle_ipc_recv(args: &SyscallArgs, caller_pid: u64) -> SyscallResult {
    let channel_id = args.arg0;
    let buffer_ptr = args.arg1;
    let buffer_len = args.arg2;
    let timeout_ms = args.arg3;

    let channel = get_manager()
        .channels
        .get_channel(ChannelId(channel_id))
        .ok_or_else(|| SyscallError::from_raw(EINVAL))?;

    let msg = channel
        .lock()
        .recv(EndpointId(args.arg4))
        .map_err(|_| SyscallError::from_raw(EPERM))?;

    Ok(msg.header.size as u64)
}

pub fn handle_ipc_call(args: &SyscallArgs, caller_pid: u64) -> SyscallResult {
    let channel_id = args.arg0;
    let request_ptr = args.arg1;
    let request_len = args.arg2;
    let reply_ptr = args.arg3;
    let reply_len = args.arg4;

    let channel = get_manager()
        .channels
        .get_channel(ChannelId(channel_id))
        .ok_or_else(|| SyscallError::from_raw(EINVAL))?;

    let msg = Message::new(super::message::MessageType::Call, caller_pid)
        .with_flags(MessageFlags::REPLY_EXPECTED | MessageFlags::BLOCKING);

    let response = channel
        .lock()
        .call(EndpointId(args.arg5), msg)
        .map_err(|_| SyscallError::from_raw(EPERM))?;

    Ok(response.header.size as u64)
}

pub fn handle_ipc_notify(args: &SyscallArgs, caller_pid: u64) -> SyscallResult {
    let notification_id = args.arg0;
    let bits = args.arg1;

    get_manager()
        .notifications
        .signal(NotificationId(notification_id), bits)
        .map_err(|_| SyscallError::from_raw(EINVAL))?;

    Ok(0)
}

pub fn handle_ipc_wait(args: &SyscallArgs, caller_pid: u64) -> SyscallResult {
    let notification_id = args.arg0;
    let mask = args.arg1;

    let notification = get_manager()
        .notifications
        .get_notification(NotificationId(notification_id))
        .ok_or_else(|| SyscallError::from_raw(EINVAL))?;

    let result = notification.wait(mask);

    Ok(result)
}

pub fn dispatch_ipc_syscall(args: &SyscallArgs, caller_pid: u64) -> SyscallResult {
    match args.num {
        SYSCALL_IPC_CREATE => handle_ipc_create(args, caller_pid),
        SYSCALL_IPC_DESTROY => handle_ipc_destroy(args),
        SYSCALL_IPC_SEND => handle_ipc_send(args, caller_pid),
        SYSCALL_IPC_RECV => handle_ipc_recv(args, caller_pid),
        _ => Err(SyscallError::from_raw(ENOSYS)),
    }
}

pub fn sys_ipc_create(target_pid: u64) -> Result<u64, IpcError> {
    let manager = get_manager();
    let caller = ProcessId::new();

    let channel_id = manager
        .channels
        .create_channel(caller, ProcessId::from_raw(target_pid))
        .map_err(|e| e)?;

    Ok(channel_id.as_u64())
}

pub fn sys_ipc_destroy(channel_id: u64) -> Result<(), IpcError> {
    get_manager()
        .channels
        .destroy_channel(ChannelId(channel_id))
        .map_err(|e| e)?;

    Ok(())
}

pub fn sys_ipc_send(channel_id: u64, endpoint_id: u64, msg: &Message) -> Result<(), IpcError> {
    let manager = get_manager();
    let channel = manager
        .channels
        .get_channel(ChannelId(channel_id))
        .ok_or(IpcError::InvalidEndpoint)?;

    channel
        .lock()
        .send(EndpointId(endpoint_id), msg.clone())
        .map_err(|_| IpcError::ChannelClosed)?;

    Ok(())
}

pub fn sys_ipc_recv(channel_id: u64, endpoint_id: u64) -> Result<Message, IpcError> {
    let manager = get_manager();
    let channel = manager
        .channels
        .get_channel(ChannelId(channel_id))
        .ok_or(IpcError::InvalidEndpoint)?;

    channel
        .lock()
        .recv(EndpointId(endpoint_id))
        .map_err(|_| IpcError::ChannelClosed)
}

pub fn sys_ipc_call(
    channel_id: u64,
    endpoint_id: u64,
    request: &Message,
) -> Result<Message, IpcError> {
    let manager = get_manager();
    let channel = manager
        .channels
        .get_channel(ChannelId(channel_id))
        .ok_or(IpcError::InvalidEndpoint)?;

    channel
        .lock()
        .call(EndpointId(endpoint_id), request.clone())
        .map_err(|_| IpcError::Timeout)
}

use super::IpcError;

pub fn sys_ipc_notify(notification_id: u64, bits: u64) -> Result<(), IpcError> {
    get_manager()
        .notifications
        .signal(NotificationId(notification_id), bits)
        .map_err(|_| IpcError::InvalidEndpoint)?;

    Ok(())
}

pub fn sys_ipc_wait(notification_id: u64, mask: u64) -> Result<u64, IpcError> {
    let notification = get_manager()
        .notifications
        .get_notification(NotificationId(notification_id))
        .ok_or(IpcError::InvalidEndpoint)?;

    let result = notification.wait(mask);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn init_test() {
        if IPC_MANAGER.get().is_none() {
            init();
        }
    }

    #[test]
    fn test_ipc_manager_initialization() {
        init_test();
        let manager = get_manager();
        assert_eq!(manager.channels.channel_count(), 0);
    }

    #[test]
    fn test_sys_ipc_create() {
        init_test();
        let pid = ProcessId::new().as_u64();

        let channel_id = sys_ipc_create(pid).unwrap();
        assert!(channel_id > 0);

        let manager = get_manager();
        assert_eq!(manager.channels.channel_count(), 1);
    }

    #[test]
    fn test_sys_ipc_destroy() {
        init_test();
        let pid = ProcessId::new().as_u64();

        let channel_id = sys_ipc_create(pid).unwrap();
        sys_ipc_destroy(channel_id).unwrap();

        let manager = get_manager();
        assert_eq!(manager.channels.channel_count(), 0);
    }

    #[test]
    fn test_sys_ipc_notify() {
        init_test();
        let process = ProcessId::new();

        let notification = get_manager().notifications.create_notification(process);
        let id = notification.id.as_u64();

        sys_ipc_notify(id, NotificationBits::BIT_0.bits()).unwrap();
        assert!(notification.is_signaled(0));
    }
}
