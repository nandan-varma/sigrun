//! Ethernet Layer
//!
//! Handles Ethernet frame processing at the data link layer.

use alloc::vec::Vec;

extern crate alloc;

/// Ethernet MAC address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub const BROADCAST: Self = Self([0xff; 6]);
    pub const ZERO: Self = Self([0x00; 6]);

    pub const fn new(addr: [u8; 6]) -> Self {
        Self(addr)
    }

    pub fn as_bytes(&self) -> &[u8; 6] {
        &self.0
    }

    pub fn is_broadcast(&self) -> bool {
        self.0 == [0xff; 6]
    }

    pub fn is_multicast(&self) -> bool {
        (self.0[0] & 0x01) != 0
    }

    pub fn is_unicast(&self) -> bool {
        !self.is_broadcast() && !self.is_multicast()
    }
}

impl core::fmt::Display for MacAddress {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

/// Ethernet frame type (EtherType)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum EtherType {
    Ipv4 = 0x0800,
    Arp = 0x0806,
    Ipv6 = 0x86DD,
    Vlan = 0x8100,
    Unknown = 0x0000,
}

impl From<u16> for EtherType {
    fn from(value: u16) -> Self {
        match value {
            0x0800 => Self::Ipv4,
            0x0806 => Self::Arp,
            0x86DD => Self::Ipv6,
            0x8100 => Self::Vlan,
            _ => Self::Unknown,
        }
    }
}

/// Ethernet frame header
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct EthernetHeader {
    pub dst: MacAddress,
    pub src: MacAddress,
    pub ether_type: u16,
}

impl EthernetHeader {
    pub const SIZE: usize = 14;

    pub fn new(dst: MacAddress, src: MacAddress, ether_type: EtherType) -> Self {
        Self {
            dst,
            src,
            ether_type: ether_type as u16,
        }
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }

        let mut dst = [0u8; 6];
        let mut src = [0u8; 6];
        dst.copy_from_slice(&data[0..6]);
        src.copy_from_slice(&data[6..12]);
        let ether_type = u16::from_be_bytes([data[12], data[13]]);

        Some(Self {
            dst: MacAddress::new(dst),
            src: MacAddress::new(src),
            ether_type,
        })
    }

    pub fn to_bytes(&self) -> [u8; 14] {
        let mut bytes = [0u8; 14];
        bytes[0..6].copy_from_slice(&self.dst.0);
        bytes[6..12].copy_from_slice(&self.src.0);
        bytes[12..14].copy_from_slice(&self.ether_type.to_be_bytes());
        bytes
    }
}

/// Ethernet frame
#[derive(Debug, Clone)]
pub struct EthernetFrame {
    pub header: EthernetHeader,
    pub payload: Vec<u8>,
}

impl EthernetFrame {
    pub fn new(dst: MacAddress, src: MacAddress, ether_type: EtherType, payload: Vec<u8>) -> Self {
        Self {
            header: EthernetHeader::new(dst, src, ether_type),
            payload,
        }
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < EthernetHeader::SIZE {
            return None;
        }

        let header = EthernetHeader::parse(data)?;
        let payload = Vec::from(&data[EthernetHeader::SIZE..]);

        Some(Self { header, payload })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(EthernetHeader::SIZE + self.payload.len());
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    pub fn size(&self) -> usize {
        EthernetHeader::SIZE + self.payload.len()
    }
}

/// Ethernet layer
pub struct EthernetLayer {
    mac_address: MacAddress,
    rx_callback: Option<fn(&EthernetFrame)>,
}

impl EthernetLayer {
    pub const fn new(mac_address: MacAddress) -> Self {
        Self {
            mac_address,
            rx_callback: None,
        }
    }

    pub fn mac_address(&self) -> MacAddress {
        self.mac_address
    }

    pub fn set_rx_callback(&mut self, callback: fn(&EthernetFrame)) {
        self.rx_callback = Some(callback);
    }

    pub fn receive(&self, data: &[u8]) -> Option<EthernetFrame> {
        let frame = EthernetFrame::parse(data)?;

        if frame.header.dst != self.mac_address
            && !frame.header.dst.is_broadcast()
            && !frame.header.dst.is_multicast()
        {
            return None;
        }

        if let Some(callback) = self.rx_callback {
            callback(&frame);
        }

        Some(frame)
    }

    pub fn send(&self, dst: MacAddress, ether_type: EtherType, payload: Vec<u8>) -> EthernetFrame {
        EthernetFrame::new(dst, self.mac_address, ether_type, payload)
    }
}

/// ARP packet
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ArpPacket {
    pub hardware_type: u16,
    pub protocol_type: u16,
    pub hardware_len: u8,
    pub protocol_len: u8,
    pub operation: u16,
    pub sender_mac: MacAddress,
    pub sender_ip: [u8; 4],
    pub target_mac: MacAddress,
    pub target_ip: [u8; 4],
}

impl ArpPacket {
    pub const REQUEST: u16 = 1;
    pub const REPLY: u16 = 2;

    pub fn new_request(sender_mac: MacAddress, sender_ip: [u8; 4], target_ip: [u8; 4]) -> Self {
        Self {
            hardware_type: 1,      // Ethernet
            protocol_type: 0x0800, // IPv4
            hardware_len: 6,
            protocol_len: 4,
            operation: Self::REQUEST,
            sender_mac,
            sender_ip,
            target_mac: MacAddress::ZERO,
            target_ip,
        }
    }

    pub fn new_reply(
        sender_mac: MacAddress,
        sender_ip: [u8; 4],
        target_mac: MacAddress,
        target_ip: [u8; 4],
    ) -> Self {
        Self {
            hardware_type: 1,
            protocol_type: 0x0800,
            hardware_len: 6,
            protocol_len: 4,
            operation: Self::REPLY,
            sender_mac,
            sender_ip,
            target_mac,
            target_ip,
        }
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 28 {
            return None;
        }

        Some(Self {
            hardware_type: u16::from_be_bytes([data[0], data[1]]),
            protocol_type: u16::from_be_bytes([data[2], data[3]]),
            hardware_len: data[4],
            protocol_len: data[5],
            operation: u16::from_be_bytes([data[6], data[7]]),
            sender_mac: MacAddress::new([data[8], data[9], data[10], data[11], data[12], data[13]]),
            sender_ip: [data[14], data[15], data[16], data[17]],
            target_mac: MacAddress::new([
                data[18], data[19], data[20], data[21], data[22], data[23],
            ]),
            target_ip: [data[24], data[25], data[26], data[27]],
        })
    }

    pub fn to_bytes(&self) -> [u8; 28] {
        let mut bytes = [0u8; 28];
        bytes[0..2].copy_from_slice(&self.hardware_type.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.protocol_type.to_be_bytes());
        bytes[4] = self.hardware_len;
        bytes[5] = self.protocol_len;
        bytes[6..8].copy_from_slice(&self.operation.to_be_bytes());
        bytes[8..14].copy_from_slice(&self.sender_mac.0);
        bytes[14..18].copy_from_slice(&self.sender_ip);
        bytes[18..24].copy_from_slice(&self.target_mac.0);
        bytes[24..28].copy_from_slice(&self.target_ip);
        bytes
    }
}
