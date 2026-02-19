//! IPv4 Layer
//!
//! Handles IPv4 packet processing at the network layer.

use alloc::vec::Vec;
extern crate alloc;

/// IPv4 address wrapper
pub type Ipv4Addr = [u8; 4];

/// IP protocol types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum IpProtocol {
    Icmp = 1,
    Igmp = 2,
    Tcp = 6,
    Udp = 17,
    Unknown = 0,
}

impl From<u8> for IpProtocol {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Icmp,
            2 => Self::Igmp,
            6 => Self::Tcp,
            17 => Self::Udp,
            _ => Self::Unknown,
        }
    }
}

/// IPv4 header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Ipv4Header {
    pub version_ihl: u8,
    pub dscp_ecn: u8,
    pub total_len: u16,
    pub identification: u16,
    pub flags_fragment: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16,
    pub src: Ipv4Addr,
    pub dst: Ipv4Addr,
}

impl Ipv4Header {
    pub const SIZE: usize = 20;

    pub fn new(src: Ipv4Addr, dst: Ipv4Addr, protocol: IpProtocol, payload_len: u16) -> Self {
        let total_len = Self::SIZE as u16 + payload_len;
        Self {
            version_ihl: 0x45, // IPv4, 5 words header
            dscp_ecn: 0,
            total_len,
            identification: 0,
            flags_fragment: 0x4000, // Don't fragment
            ttl: 64,
            protocol: protocol as u8,
            checksum: 0,
            src,
            dst,
        }
    }

    pub fn version(&self) -> u8 {
        (self.version_ihl >> 4) & 0x0f
    }

    pub fn ihl(&self) -> u8 {
        self.version_ihl & 0x0f
    }

    pub fn header_len(&self) -> usize {
        (self.ihl() * 4) as usize
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            version_ihl: data[0],
            dscp_ecn: data[1],
            total_len: u16::from_be_bytes([data[2], data[3]]),
            identification: u16::from_be_bytes([data[4], data[5]]),
            flags_fragment: u16::from_be_bytes([data[6], data[7]]),
            ttl: data[8],
            protocol: data[9],
            checksum: u16::from_be_bytes([data[10], data[11]]),
            src: [data[12], data[13], data[14], data[15]],
            dst: [data[16], data[17], data[18], data[19]],
        })
    }

    pub fn to_bytes(&self) -> [u8; 20] {
        let mut bytes = [0u8; 20];
        bytes[0] = self.version_ihl;
        bytes[1] = self.dscp_ecn;
        bytes[2..4].copy_from_slice(&self.total_len.to_be_bytes());
        bytes[4..6].copy_from_slice(&self.identification.to_be_bytes());
        bytes[6..8].copy_from_slice(&self.flags_fragment.to_be_bytes());
        bytes[8] = self.ttl;
        bytes[9] = self.protocol;
        bytes[10..12].copy_from_slice(&self.checksum.to_be_bytes());
        bytes[12..16].copy_from_slice(&self.src);
        bytes[16..20].copy_from_slice(&self.dst);
        bytes
    }

    pub fn calc_checksum(&mut self) {
        self.checksum = 0;
        let bytes = self.to_bytes();
        let mut sum: u32 = 0;

        for i in (0..20).step_by(2) {
            sum += u16::from_be_bytes([bytes[i], bytes[i + 1]]) as u32;
        }

        while sum >> 16 != 0 {
            sum = (sum & 0xffff) + (sum >> 16);
        }

        self.checksum = !(sum as u16);
    }

    pub fn verify_checksum(&self) -> bool {
        let bytes = self.to_bytes();
        let mut sum: u32 = 0;

        for i in (0..20).step_by(2) {
            sum += u16::from_be_bytes([bytes[i], bytes[i + 1]]) as u32;
        }

        while sum >> 16 != 0 {
            sum = (sum & 0xffff) + (sum >> 16);
        }

        !(sum as u16) == 0
    }
}

/// IPv4 packet
#[derive(Debug, Clone)]
pub struct Ipv4Packet {
    pub header: Ipv4Header,
    pub payload: Vec<u8>,
}

impl Ipv4Packet {
    pub fn new(src: Ipv4Addr, dst: Ipv4Addr, protocol: IpProtocol, payload: Vec<u8>) -> Self {
        let mut header = Ipv4Header::new(src, dst, protocol, payload.len() as u16);
        header.calc_checksum();

        Self { header, payload }
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        let header = Ipv4Header::parse(data)?;
        let header_len = header.header_len();
        let payload = Vec::from(&data[header_len..header.total_len as usize]);

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

/// ICMP packet
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct IcmpPacket {
    pub icmp_type: u8,
    pub code: u8,
    pub checksum: u16,
    pub rest: u32,
}

impl IcmpPacket {
    pub const ECHO_REPLY: u8 = 0;
    pub const ECHO_REQUEST: u8 = 8;
    pub const DESTINATION_UNREACHABLE: u8 = 3;
    pub const TIME_EXCEEDED: u8 = 11;

    pub fn new_echo_request(identifier: u16, sequence: u16) -> Self {
        Self {
            icmp_type: Self::ECHO_REQUEST,
            code: 0,
            checksum: 0,
            rest: ((identifier as u32) << 16) | (sequence as u32),
        }
    }

    pub fn new_echo_reply(identifier: u16, sequence: u16) -> Self {
        Self {
            icmp_type: Self::ECHO_REPLY,
            code: 0,
            checksum: 0,
            rest: ((identifier as u32) << 16) | (sequence as u32),
        }
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 8 {
            return None;
        }

        Some(Self {
            icmp_type: data[0],
            code: data[1],
            checksum: u16::from_be_bytes([data[2], data[3]]),
            rest: u32::from_be_bytes([data[4], data[5], data[6], data[7]]),
        })
    }

    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0] = self.icmp_type;
        bytes[1] = self.code;
        bytes[2..4].copy_from_slice(&self.checksum.to_be_bytes());
        bytes[4..8].copy_from_slice(&self.rest.to_be_bytes());
        bytes
    }

    pub fn identifier(&self) -> u16 {
        (self.rest >> 16) as u16
    }

    pub fn sequence(&self) -> u16 {
        self.rest as u16
    }
}

/// IPv4 layer
pub struct Ipv4Layer {
    address: Ipv4Addr,
    rx_callback: Option<fn(&Ipv4Packet)>,
}

impl Ipv4Layer {
    pub const fn new(address: Ipv4Addr) -> Self {
        Self {
            address,
            rx_callback: None,
        }
    }

    pub fn address(&self) -> Ipv4Addr {
        self.address
    }

    pub fn set_rx_callback(&mut self, callback: fn(&Ipv4Packet)) {
        self.rx_callback = Some(callback);
    }

    pub fn receive(&self, data: &[u8]) -> Option<Ipv4Packet> {
        let packet = Ipv4Packet::parse(data)?;

        if !packet.header.verify_checksum() {
            return None;
        }

        if packet.header.dst != self.address && packet.header.dst != [255, 255, 255, 255] {
            return None;
        }

        if let Some(callback) = self.rx_callback {
            callback(&packet);
        }

        Some(packet)
    }

    pub fn send(&self, dst: Ipv4Addr, protocol: IpProtocol, payload: Vec<u8>) -> Ipv4Packet {
        Ipv4Packet::new(self.address, dst, protocol, payload)
    }

    pub fn send_icmp_echo_reply(&self, request: &IcmpPacket) -> Option<Ipv4Packet> {
        if request.icmp_type != IcmpPacket::ECHO_REQUEST {
            return None;
        }

        let reply = IcmpPacket::new_echo_reply(request.identifier(), request.sequence());
        let payload = reply.to_bytes().to_vec();

        Some(self.send([0, 0, 0, 0], IpProtocol::Icmp, payload))
    }
}
