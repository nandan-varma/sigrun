//! Network Stack Server
//!
//! Provides TCP/IP network stack for the SIGRUN operating system.
//! Implements Ethernet, IPv4, TCP, and UDP layers.

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate alloc;
extern crate common;
extern crate syscall_api;

pub mod ethernet;
pub mod ipv4;
pub mod tcp;
pub mod udp;
pub mod socket;
pub mod server;

use alloc::vec::Vec;
use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};
use ethernet::{EthernetLayer, MacAddress};
use ipv4::Ipv4Layer;
use tcp::TcpLayer;
use udp::UdpLayer;
use socket::SocketTable;
use server::NetServer;
use syscall_api::{SyscallArgs, SYSCALL_YIELD, SYSCALL_SLEEP};

#[global_allocator]
static ALLOCATOR: BumpAllocator = BumpAllocator::new();

struct BumpAllocator {
    offset: AtomicUsize,
    size: usize,
}

impl BumpAllocator {
    const fn new() -> Self {
        Self { 
            offset: AtomicUsize::new(0), 
            size: 64 * 1024 
        }
    }
}

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let start = (self.offset.load(Ordering::Relaxed) + layout.align() - 1) & !(layout.align() - 1);
        let end = start + layout.size();
        if end > self.size {
            core::ptr::null_mut()
        } else {
            self.offset.store(end, Ordering::Relaxed);
            start as *mut u8
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

pub fn main() {
    println("SIGRUN Network Stack v0.1");
    
    let eth = EthernetLayer::new(MacAddress::new([0x52, 0x54, 0x00, 0x12, 0x34, 0x56]));
    let ipv4 = Ipv4Layer::new([10, 0, 2, 15]);
    let tcp = TcpLayer::new();
    let udp = UdpLayer::new();
    let sockets = SocketTable::new();
    
    let mut server = NetServer::new(eth, ipv4, tcp, udp, sockets);
    
    println("Network stack initialized");
    println("MAC: 52:54:00:12:34:56");
    println("IP: 10.0.2.15");
    
    server.run()
}

fn println(msg: &str) {
    let bytes = msg.as_bytes();
    let args = SyscallArgs::new(4)
        .with_3args(1, bytes.as_ptr() as u64, bytes.len() as u64);
    unsafe {
        syscall_api::syscall(args).ok();
    }
}

fn yield_now() {
    let args = SyscallArgs::new(SYSCALL_YIELD);
    unsafe {
        syscall_api::syscall(args).ok();
    }
}

fn sleep_ms(ms: u64) {
    let args = SyscallArgs::new(SYSCALL_SLEEP).with_arg0(ms);
    unsafe {
        syscall_api::syscall(args).ok();
    }
}
