//! Socket API
//!
//! Provides socket abstraction for the network stack.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

extern crate alloc;

/// Socket address
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SocketAddr {
    pub addr: [u8; 4],
    pub port: u16,
}

impl SocketAddr {
    pub const fn new(addr: [u8; 4], port: u16) -> Self {
        Self { addr, port }
    }

    pub const fn loopback(port: u16) -> Self {
        Self {
            addr: [127, 0, 0, 1],
            port,
        }
    }

    pub const fn any(port: u16) -> Self {
        Self {
            addr: [0, 0, 0, 0],
            port,
        }
    }

    pub fn from_raw(raw: u32, port: u16) -> Self {
        Self {
            addr: [
                (raw & 0xff) as u8,
                ((raw >> 8) & 0xff) as u8,
                ((raw >> 16) & 0xff) as u8,
                ((raw >> 24) & 0xff) as u8,
            ],
            port,
        }
    }

    pub fn to_u32(&self) -> u32 {
        (self.addr[0] as u32)
            | ((self.addr[1] as u32) << 8)
            | ((self.addr[2] as u32) << 16)
            | ((self.addr[3] as u32) << 24)
    }
}

/// Socket domain
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Unix = 1,
    Inet = 2,
    Inet6 = 10,
}

/// Socket type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SockType {
    Stream = 1,
    Dgram = 2,
    Raw = 3,
}

/// Socket protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Tcp = 6,
    Udp = 17,
    Icmp = 1,
}

/// Socket state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Closed,
    Opening,
    Connected,
    Listening,
    Bound,
}

/// Socket
pub struct Socket {
    pub domain: Domain,
    pub sock_type: SockType,
    pub protocol: Protocol,
    pub state: SocketState,
    pub local_addr: Option<SocketAddr>,
    pub peer_addr: Option<SocketAddr>,
    pub backlog: usize,
    pub rx_buffer: Vec<u8>,
    pub tx_buffer: Vec<u8>,
}

impl Socket {
    pub fn new(domain: Domain, sock_type: SockType, protocol: Protocol) -> Self {
        Self {
            domain,
            sock_type,
            protocol,
            state: SocketState::Closed,
            local_addr: None,
            peer_addr: None,
            backlog: 128,
            rx_buffer: Vec::new(),
            tx_buffer: Vec::new(),
        }
    }

    pub fn bind(&mut self, addr: SocketAddr) {
        self.local_addr = Some(addr);
        self.state = SocketState::Bound;
    }

    pub fn listen(&mut self, backlog: usize) {
        self.backlog = backlog;
        self.state = SocketState::Listening;
    }

    pub fn connect(&mut self, addr: SocketAddr) {
        self.peer_addr = Some(addr);
        self.state = SocketState::Connected;
    }

    pub fn accept(&mut self) -> Option<Socket> {
        if self.state != SocketState::Listening {
            return None;
        }

        let mut new_socket = Socket::new(self.domain, self.sock_type, self.protocol);
        new_socket.state = SocketState::Connected;
        new_socket.local_addr = self.local_addr;
        new_socket.peer_addr = self.peer_addr;

        Some(new_socket)
    }

    pub fn send(&mut self, data: &[u8]) -> usize {
        self.tx_buffer.extend_from_slice(data);
        data.len()
    }

    pub fn recv(&mut self, buf: &mut [u8]) -> usize {
        let to_read = buf.len().min(self.rx_buffer.len());
        buf[..to_read].copy_from_slice(&self.rx_buffer[..to_read]);
        self.rx_buffer.drain(..to_read);
        to_read
    }

    pub fn close(&mut self) {
        self.state = SocketState::Closed;
        self.rx_buffer.clear();
        self.tx_buffer.clear();
    }
}

/// Socket table
pub struct SocketTable {
    sockets: BTreeMap<usize, Socket>,
    next_id: usize,
}

impl Default for SocketTable {
    fn default() -> Self {
        Self::new()
    }
}

impl SocketTable {
    pub const fn new() -> Self {
        Self {
            sockets: BTreeMap::new(),
            next_id: 3,
        }
    }

    pub fn socket(&mut self, domain: Domain, sock_type: SockType, protocol: Protocol) -> usize {
        let id = self.next_id;
        self.next_id += 1;

        let socket = Socket::new(domain, sock_type, protocol);
        self.sockets.insert(id, socket);

        id
    }

    pub fn bind(&mut self, id: usize, addr: SocketAddr) -> Result<(), i32> {
        if let Some(socket) = self.sockets.get_mut(&id) {
            socket.bind(addr);
            Ok(())
        } else {
            Err(9) // EBADF
        }
    }

    pub fn listen(&mut self, id: usize, backlog: usize) -> Result<(), i32> {
        if let Some(socket) = self.sockets.get_mut(&id) {
            socket.listen(backlog);
            Ok(())
        } else {
            Err(9) // EBADF
        }
    }

    pub fn connect(&mut self, id: usize, addr: SocketAddr) -> Result<(), i32> {
        if let Some(socket) = self.sockets.get_mut(&id) {
            socket.connect(addr);
            Ok(())
        } else {
            Err(9) // EBADF
        }
    }

    pub fn accept(&mut self, id: usize) -> Result<usize, i32> {
        if let Some(socket) = self.sockets.get_mut(&id) {
            if let Some(new_socket) = socket.accept() {
                let new_id = self.next_id;
                self.next_id += 1;
                self.sockets.insert(new_id, new_socket);
                Ok(new_id)
            } else {
                Err(11) // EAGAIN
            }
        } else {
            Err(9) // EBADF
        }
    }

    pub fn send(&mut self, id: usize, data: &[u8]) -> Result<usize, i32> {
        if let Some(socket) = self.sockets.get_mut(&id) {
            Ok(socket.send(data))
        } else {
            Err(9) // EBADF
        }
    }

    pub fn recv(&mut self, id: usize, buf: &mut [u8]) -> Result<usize, i32> {
        if let Some(socket) = self.sockets.get_mut(&id) {
            Ok(socket.recv(buf))
        } else {
            Err(9) // EBADF
        }
    }

    pub fn close(&mut self, id: usize) -> Result<(), i32> {
        if let Some(socket) = self.sockets.get_mut(&id) {
            socket.close();
            self.sockets.remove(&id);
            Ok(())
        } else {
            Err(9) // EBADF
        }
    }

    pub fn get(&self, id: usize) -> Option<&Socket> {
        self.sockets.get(&id)
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut Socket> {
        self.sockets.get_mut(&id)
    }

    pub fn count(&self) -> usize {
        self.sockets.len()
    }
}
