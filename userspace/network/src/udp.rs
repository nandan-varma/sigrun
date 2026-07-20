//! UDP Layer
//!
//! Handles UDP datagram processing at the transport layer.

use alloc::vec::Vec;

extern crate alloc;

/// UDP header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UdpHeader {
    pub src_port: u16,
    pub dst_port: u16,
    pub length: u16,
    pub checksum: u16,
}

impl UdpHeader {
    pub const SIZE: usize = 8;

    pub fn new(src_port: u16, dst_port: u16, payload_len: u16) -> Self {
        Self {
            src_port: src_port.to_be(),
            dst_port: dst_port.to_be(),
            length: (Self::SIZE as u16 + payload_len).to_be(),
            checksum: 0,
        }
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < Self::SIZE {
            return None;
        }

        Some(Self {
            src_port: u16::from_be_bytes([data[0], data[1]]),
            dst_port: u16::from_be_bytes([data[2], data[3]]),
            length: u16::from_be_bytes([data[4], data[5]]),
            checksum: u16::from_be_bytes([data[6], data[7]]),
        })
    }

    pub fn to_bytes(&self) -> [u8; 8] {
        let mut bytes = [0u8; 8];
        bytes[0..2].copy_from_slice(&self.src_port.to_be_bytes());
        bytes[2..4].copy_from_slice(&self.dst_port.to_be_bytes());
        bytes[4..6].copy_from_slice(&self.length.to_be_bytes());
        bytes[6..8].copy_from_slice(&self.checksum.to_be_bytes());
        bytes
    }
}

/// UDP datagram
#[derive(Debug, Clone)]
pub struct UdpDatagram {
    pub header: UdpHeader,
    pub payload: Vec<u8>,
}

impl UdpDatagram {
    pub fn new(src_port: u16, dst_port: u16, payload: Vec<u8>) -> Self {
        Self {
            header: UdpHeader::new(src_port, dst_port, payload.len() as u16),
            payload,
        }
    }

    pub fn parse(data: &[u8]) -> Option<Self> {
        let header = UdpHeader::parse(data)?;
        let payload_len = u16::from_be(header.length) as usize - UdpHeader::SIZE;
        let payload = Vec::from(&data[UdpHeader::SIZE..UdpHeader::SIZE + payload_len]);

        Some(Self { header, payload })
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(UdpHeader::SIZE + self.payload.len());
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(&self.payload);
        bytes
    }

    pub fn size(&self) -> usize {
        UdpHeader::SIZE + self.payload.len()
    }

    pub fn src_port(&self) -> u16 {
        u16::from_be(self.header.src_port)
    }

    pub fn dst_port(&self) -> u16 {
        u16::from_be(self.header.dst_port)
    }
}

/// UDP socket
pub struct UdpSocket {
    pub local_port: u16,
    pub remote_addr: Option<[u8; 4]>,
    pub remote_port: Option<u16>,
    pub bound: bool,
}

impl Default for UdpSocket {
    fn default() -> Self {
        Self::new()
    }
}

impl UdpSocket {
    pub const fn new() -> Self {
        Self {
            local_port: 0,
            remote_addr: None,
            remote_port: None,
            bound: false,
        }
    }

    pub fn bind(&mut self, port: u16) {
        self.local_port = port;
        self.bound = true;
    }

    pub fn connect(&mut self, addr: [u8; 4], port: u16) {
        self.remote_addr = Some(addr);
        self.remote_port = Some(port);
    }

    pub fn send_to(&self, data: &[u8], _addr: [u8; 4], port: u16) -> Option<UdpDatagram> {
        if !self.bound {
            return None;
        }

        Some(UdpDatagram::new(self.local_port, port, Vec::from(data)))
    }

    pub fn recv_from(&mut self, data: &[u8]) -> Option<(u16, u16, Vec<u8>)> {
        let datagram = UdpDatagram::parse(data)?;

        Some((datagram.src_port(), datagram.dst_port(), datagram.payload))
    }
}

/// UDP layer
pub struct UdpLayer {
    sockets: alloc::collections::BTreeMap<u16, UdpSocket>,
    rx_callback: Option<fn(&UdpDatagram)>,
}

impl Default for UdpLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl UdpLayer {
    pub const fn new() -> Self {
        Self {
            sockets: alloc::collections::BTreeMap::new(),
            rx_callback: None,
        }
    }

    pub fn set_rx_callback(&mut self, callback: fn(&UdpDatagram)) {
        self.rx_callback = Some(callback);
    }

    pub fn receive(&self, data: &[u8]) -> Option<UdpDatagram> {
        let datagram = UdpDatagram::parse(data)?;

        if let Some(callback) = self.rx_callback {
            callback(&datagram);
        }

        Some(datagram)
    }

    pub fn send(
        &self,
        src_port: u16,
        _dst_addr: [u8; 4],
        dst_port: u16,
        payload: Vec<u8>,
    ) -> UdpDatagram {
        UdpDatagram::new(src_port, dst_port, payload)
    }

    pub fn bind(&mut self, port: u16) -> &mut UdpSocket {
        let socket = UdpSocket::new();
        let mut socket = socket;
        socket.bind(port);
        self.sockets.insert(port, socket);
        self.sockets.get_mut(&port).unwrap()
    }

    pub fn get_socket(&self, port: u16) -> Option<&UdpSocket> {
        self.sockets.get(&port)
    }

    pub fn get_socket_mut(&mut self, port: u16) -> Option<&mut UdpSocket> {
        self.sockets.get_mut(&port)
    }
}
