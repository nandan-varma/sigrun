//! Network Server Implementation
//!
//! Handles network I/O and routes packets between layers.

use crate::ethernet::{EtherType, EthernetFrame, EthernetLayer};
use crate::ipv4::{IpProtocol, Ipv4Layer};
use crate::socket::{SocketAddr, SocketTable};
use crate::tcp::{TcpFlags, TcpLayer, TcpSegment};
use crate::udp::{UdpDatagram, UdpLayer};
use syscall_api::{syscall, SyscallArgs};

fn println(msg: &str) {
    let bytes = msg.as_bytes();
    let args = SyscallArgs::new(4).with_3args(1, bytes.as_ptr() as u64, bytes.len() as u64);
    unsafe {
        syscall(args).ok();
    }
}

pub struct NetServer {
    eth: EthernetLayer,
    ipv4: Ipv4Layer,
    tcp: TcpLayer,
    udp: UdpLayer,
    sockets: SocketTable,
}

impl NetServer {
    pub fn new(
        eth: EthernetLayer,
        ipv4: Ipv4Layer,
        tcp: TcpLayer,
        udp: UdpLayer,
        sockets: SocketTable,
    ) -> Self {
        Self {
            eth,
            ipv4,
            tcp,
            udp,
            sockets,
        }
    }

    pub fn run(&mut self) -> ! {
        println("Network server running...");

        loop {
            self.process_outgoing();
            self.tick();
            crate::yield_now();
        }
    }

    fn process_incoming(&mut self, _data: &[u8]) {
        if let Some(frame) = self.eth.receive(_data) {
            match frame.header.ether_type.into() {
                EtherType::Ipv4 => {
                    self.process_ipv4(&frame.payload);
                }
                EtherType::Arp => {
                    self.process_arp(&frame.payload);
                }
                _ => {}
            }
        }
    }

    fn process_ipv4(&mut self, data: &[u8]) {
        if let Some(packet) = self.ipv4.receive(data) {
            match packet.header.protocol.into() {
                IpProtocol::Tcp => {
                    self.process_tcp(&packet.payload);
                }
                IpProtocol::Udp => {
                    self.process_udp(&packet.payload);
                }
                IpProtocol::Icmp => {
                    self.process_icmp(&packet.payload);
                }
                _ => {}
            }
        }
    }

    fn process_tcp(&mut self, data: &[u8]) {
        if let Some(segment) = self.tcp.receive(data) {
            let src_port = u16::from_be(segment.header.src_port);
            let dst_port = u16::from_be(segment.header.dst_port);

            let flags = segment.header.flags();

            if flags.contains(TcpFlags::SYN) {
                println("TCP SYN received");
            } else if flags.contains(TcpFlags::ACK) {
                println("TCP ACK received");
            } else if flags.contains(TcpFlags::FIN) {
                println("TCP FIN received");
            }
        }
    }

    fn process_udp(&mut self, data: &[u8]) {
        if let Some(datagram) = self.udp.receive(data) {
            let src_port = datagram.src_port();
            let dst_port = datagram.dst_port();

            if dst_port == 53 {
                println("DNS request received");
            }
        }
    }

    fn process_icmp(&mut self, data: &[u8]) {
        if let Some(icmp) = crate::ipv4::IcmpPacket::parse(data) {
            match icmp.icmp_type {
                8 => {
                    println("ICMP echo request");
                }
                0 => {
                    println("ICMP echo reply");
                }
                _ => {}
            }
        }
    }

    fn process_arp(&mut self, data: &[u8]) {
        if let Some(arp) = crate::ethernet::ArpPacket::parse(data) {
            if arp.operation == crate::ethernet::ArpPacket::REQUEST {
                println("ARP request received");
            }
        }
    }

    fn process_outgoing(&mut self) {
        self.flush_tcp();
        self.flush_udp();
    }

    fn flush_tcp(&mut self) {
        // In a real implementation, this would send buffered TCP data
    }

    fn flush_udp(&mut self) {
        // In a real implementation, this would send buffered UDP data
    }

    pub fn socket(&mut self, domain: i32, sock_type: i32, protocol: i32) -> Result<usize, i32> {
        let domain = match domain {
            1 => crate::socket::Domain::Unix,
            2 => crate::socket::Domain::Inet,
            10 => crate::socket::Domain::Inet6,
            _ => return Err(22), // EINVAL
        };

        let sock_type = match sock_type {
            1 => crate::socket::SockType::Stream,
            2 => crate::socket::SockType::Dgram,
            3 => crate::socket::SockType::Raw,
            _ => return Err(22), // EINVAL
        };

        let protocol = match protocol {
            6 => crate::socket::Protocol::Tcp,
            17 => crate::socket::Protocol::Udp,
            1 => crate::socket::Protocol::Icmp,
            0 => crate::socket::Protocol::Tcp,
            _ => return Err(22), // EINVAL
        };

        Ok(self.sockets.socket(domain, sock_type, protocol))
    }

    pub fn bind(&mut self, fd: usize, addr: SocketAddr) -> Result<(), i32> {
        self.sockets.bind(fd, addr)
    }

    pub fn listen(&mut self, fd: usize, backlog: i32) -> Result<(), i32> {
        self.sockets.listen(fd, backlog as usize)
    }

    pub fn connect(&mut self, fd: usize, addr: SocketAddr) -> Result<(), i32> {
        self.sockets.connect(fd, addr)
    }

    pub fn accept(&mut self, fd: usize) -> Result<usize, i32> {
        self.sockets.accept(fd)
    }

    pub fn send(&mut self, fd: usize, data: &[u8]) -> Result<usize, i32> {
        self.sockets.send(fd, data)
    }

    pub fn recv(&mut self, fd: usize, buf: &mut [u8]) -> Result<usize, i32> {
        self.sockets.recv(fd, buf)
    }

    pub fn close(&mut self, fd: usize) -> Result<(), i32> {
        self.sockets.close(fd)
    }

    fn tick(&mut self) {
        // Handle timeouts, retransmissions, etc.
    }
}
