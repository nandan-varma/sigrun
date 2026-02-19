//! IPC utilities for userspace

use syscall_api::{SYSCALL_IPC_RECV, SYSCALL_IPC_SEND, SyscallArgs, SyscallError};

pub const MAX_INLINE_PAYLOAD: usize = 256;
pub const MAX_INLINE_CAPS: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MsgType {
    Call = 0,
    Send = 1,
    Signal = 2,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MessageHeader {
    pub msg_type: MsgType,
    pub flags: u8,
    pub cap_count: u8,
    pub payload_len: u16,
    pub sender_pid: u64,
    pub reply_to: u64,
}

impl MessageHeader {
    pub const fn new(msg_type: MsgType) -> Self {
        Self {
            msg_type,
            flags: 0,
            cap_count: 0,
            payload_len: 0,
            sender_pid: 0,
            reply_to: 0,
        }
    }

    pub const fn with_payload_len(mut self, len: u16) -> Self {
        self.payload_len = len;
        self
    }

    pub const fn blocking(mut self) -> Self {
        self.flags |= 0x01;
        self
    }

    pub fn with_caps(mut self, count: u8) -> Self {
        self.cap_count = if count > MAX_INLINE_CAPS as u8 {
            MAX_INLINE_CAPS as u8
        } else {
            count
        };
        self.flags |= 0x02;
        self
    }
}

#[repr(C)]
pub struct Message {
    pub header: MessageHeader,
    pub caps: [u64; MAX_INLINE_CAPS],
    pub payload: [u8; MAX_INLINE_PAYLOAD],
}

impl Message {
    pub const fn new(msg_type: MsgType) -> Self {
        Self {
            header: MessageHeader::new(msg_type),
            caps: [0; MAX_INLINE_CAPS],
            payload: [0; MAX_INLINE_PAYLOAD],
        }
    }

    pub fn call() -> Self {
        let mut msg = Self::new(MsgType::Call);
        msg.header.flags |= 0x01;
        msg
    }

    pub fn send() -> Self {
        Self::new(MsgType::Send)
    }

    pub fn signal() -> Self {
        Self::new(MsgType::Signal)
    }

    pub fn with_payload(mut self, data: &[u8]) -> Self {
        let len = data.len().min(MAX_INLINE_PAYLOAD);
        self.payload[..len].copy_from_slice(&data[..len]);
        self.header.payload_len = len as u16;
        self
    }

    pub fn with_cap(mut self, cap: u64) -> Self {
        for i in 0..MAX_INLINE_CAPS {
            if self.caps[i] == 0 {
                self.caps[i] = cap;
                self.header.cap_count += 1;
                self.header.flags |= 0x02;
                break;
            }
        }
        self
    }

    pub fn payload(&self) -> &[u8] {
        let len = self.header.payload_len as usize;
        &self.payload[..len]
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Endpoint {
    pub target_pid: u64,
    pub channel_id: u64,
}

impl Endpoint {
    pub const fn new(target_pid: u64, channel_id: u64) -> Self {
        Self {
            target_pid,
            channel_id,
        }
    }

    pub fn send(&self, msg: &Message) -> Result<(), SyscallError> {
        unsafe {
            let args = SyscallArgs::new(SYSCALL_IPC_SEND).with_3args(
                self.target_pid,
                self.channel_id,
                msg as *const Message as u64,
            );
            syscall_api::syscall(args)?;
        }
        Ok(())
    }

    pub fn recv(&self, msg: &mut Message, blocking: bool) -> Result<(), SyscallError> {
        unsafe {
            let flags = if blocking { 1u64 } else { 0u64 };
            let args = SyscallArgs::new(SYSCALL_IPC_RECV).with_4args(
                self.target_pid,
                self.channel_id,
                msg as *mut Message as u64,
                flags,
            );
            syscall_api::syscall(args)?;
        }
        Ok(())
    }

    pub fn call(&self, msg: &mut Message) -> Result<(), SyscallError> {
        msg.header.msg_type = MsgType::Call;
        msg.header.flags |= 0x01;
        self.send(msg)?;
        self.recv(msg, true)
    }
}
