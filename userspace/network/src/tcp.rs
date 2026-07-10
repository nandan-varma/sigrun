//! TCP Layer
//!
//! Handles TCP segment processing at the transport layer.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use bitflags::bitflags;

extern crate alloc;

/// TCP port
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Port(u16);

impl Port {
    pub const fn new(port: u16) -> Self {
        Self(port)
    }

    pub const fn raw(self) -> u16 {
        self.0
    }

    pub const HTTP: Self = Self(80);
    pub const HTTPS: Self = Self(443);
    pub const SSH: Self = Self(22);
    pub const DNS: Self = Self(53);
}

/// TCP flags
bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct TcpFlags: u8 {
        const FIN = 0x01;
        const SYN = 0x02;
        const RST = 0x04;
        const PSH = 0x08;
        const ACK = 0x10;
        const URG = 0x20;
        const ECE = 0x40;
        const CWR = 0x80;
    }
}

/// TCP header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TcpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub seq_num: u32,
    pub ack_num: u32,
    pub data_offset_flags: u16,
    pub window: u16,
    pub checksum: u16,
    pub urgent: u16,
}

impl TcpHeader {
    pub const SIZE: usize = 20;

    pub fn new(
        src_port: Port,
        dst_port: Port,
        seq_num: u32,
        ack_num: u32,
        flags: TcpFlags,
    ) -> Self {
        Self {
            src_port: src_port.raw().to_be(),
            dst_port: dst_port.raw().to_be(),
            seq_num: seq_num.to_be(),
            ack_num: ack_num.to_be(),
            data_offset_flags: ((5u8 as u16) << 12) | (flags.bits() as u16),
            window: 65535,
            checksum: 0,
            urgent: 0,
        }
    }

    pub fn data_offset(&self) -> u8 {
        ((self.data_offset_flags >> 12) & 0x0f) as u8
    }

    pub fn header_len(&self) -> usize {
        (self.data_offset() * 4) as usize
    }

    pub fn flags(&self) -> TcpFlags {
        TcpFlags::from_bits_truncate((self.data_offset_flags & 0x3f) as u8)
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            seq_num: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
            ack_num: u32::from_be_bytes([data[8], data[9], data[10], data[11]]),
            data_offset_flags: u16::from_be_bytes([data[12], data[13]]),
            window: u16::from_be_bytes([data[14], data[15]]),
            checksum: u16::from_be_bytes([data[16], data[17]]),
            urgent: u16::from_be_bytes([data[18], data[19]]),
        })
    }

    pub fn to_bytes(&self) -> [u8; 20] {
        let mut bytes = [0u8; 20];
        bytes[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.seq_num.to_be_bytes());
        bytes[8..12].copy_from_slice(&self.ack_num.to_be_bytes());
        bytes[12..14].copy_from_slice(&self.data_offset_flags.to_be_bytes());
        bytes[14..16].copy_from_slice(&self.window.to_be_bytes());
        bytes[16..18].copy_from_slice(&self.checksum.to_be_bytes());
        bytes[18..20].copy_from_slice(&self.urgent.to_be_bytes());
        bytes
    }
}

/// TCP segment
#[derive(Debug, Clone)]
pub struct TcpSegment {
    pub header: TcpHeader,
    pub payload: Vec<u8>,
}

impl TcpSegment {
    pub fn new(
        src_port: Port,
        dst_port: Port,
        seq_num: u32,
        ack_num: u32,
        flags: TcpFlags,
        payload: Vec<u8>,
    ) -> Self {
        Self {
            header: TcpHeader::new(src_port, dst_port, seq_num, ack_num, flags),
            payload,
        }
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        let header = TcpHeader::parse(data)?;
        let header_len = header.header_len();
        let payload = Vec::from(&data[header_len..]);

        Some(Self { header, payload })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.header.header_len() + self.payload.len());
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    pub fn size(&self) -> usize {
        self.header.header_len() + self.payload.len()
    }
}

/// TCP connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TcpState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

/// TCP connection
pub struct TcpConnection {
    pub local_port: Port,
    pub remote_addr: [u8; 4],
    pub remote_port: Port,
    pub state: TcpState,
    pub seq_num: u32,
    pub ack_num: u32,
    pub remote_window: u16,
    pub last_update: u64,
}

impl TcpConnection {
    pub fn new(local_port: Port, remote_addr: [u8; 4], remote_port: Port) -> Self {
        Self {
            local_port,
            remote_addr,
            remote_port,
            state: TcpState::Closed,
            seq_num: 0,
            ack_num: 0,
            remote_window: 65535,
            last_update: 0,
        }
    }

    pub fn src_key(&self) -> (Port, [u8; 4], Port) {
        (self.local_port, self.remote_addr, self.remote_port)
    }
}

/// TCP layer
pub struct TcpLayer {
    connections: BTreeMap<(Port, [u8; 4], Port), TcpConnection>,
    next_port: Port,
    rx_callback: Option<fn(&TcpSegment)>,
}

impl TcpLayer {
    pub const fn new() -> Self {
        Self {
            connections: BTreeMap::new(),
            next_port: Port::new(49152),
            rx_callback: None,
        }
    }

    pub fn set_rx_callback(&mut self, callback: fn(&TcpSegment)) {
        self.rx_callback = Some(callback);
    }

    pub fn receive(&self, data: &[u8]) -> Option<TcpSegment> {
        let segment = TcpSegment::parse(data)?;

        let src_port = Port::new(u16::from_be(segment.header.src_port));
        let dst_port = Port::new(u16::from_be(segment.header.dst_port));

        if let Some(callback) = self.rx_callback {
            callback(&segment);
        }

        Some(segment)
    }

    pub fn send(
        &mut self,
        src_port: Port,
        dst_addr: [u8; 4],
        dst_port: Port,
        seq_num: u32,
        ack_num: u32,
        flags: TcpFlags,
        payload: Vec<u8>,
    ) -> TcpSegment {
        TcpSegment::new(src_port, dst_port, seq_num, ack_num, flags, payload)
    }

    pub fn connect(
        &mut self,
        local_port: Port,
        remote_addr: [u8; 4],
        remote_port: Port,
    ) -> Option<&TcpConnection> {
        let key = (local_port, remote_addr, remote_port);
        let mut conn = TcpConnection::new(local_port, remote_addr, remote_port);
        conn.state = TcpState::SynSent;
        conn.seq_num = rand_u32();

        self.connections.insert(key, conn);
        self.connections.get(&key)
    }

    pub fn listen(&mut self, port: Port) {
        let conn = TcpConnection::new(port, [0, 0, 0, 0], Port::new(0));
        self.connections
            .insert((port, [0, 0, 0, 0], Port::new(0)), conn);
    }

    pub fn get_connection(&self, key: &(Port, [u8; 4], Port)) -> Option<&TcpConnection> {
        self.connections.get(key)
    }

    pub fn alloc_port(&mut self) -> Port {
        let port = self.next_port;
        self.next_port = Port::new(port.raw().wrapping_add(1));
        port
    }
}

fn rand_u32() -> u32 {
    use core::time::Duration;
    let ticks = Duration::ZERO.as_nanos() as u32;
    ticks.wrapping_mul(1103515245).wrapping_add(12345)
}
