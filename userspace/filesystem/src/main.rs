//! Filesystem Server
//!
//! Provides VFS and filesystem services for the SIGRUN operating system.

#![no_std]
#![no_main]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate common;
extern crate syscall_api;

pub mod immutable;
pub mod request;
pub mod server;
pub mod vfs;

use core::panic::PanicInfo;
use immutable::ImmutableFs;
use server::FsServer;
use syscall_api::{SYSCALL_YIELD, SyscallArgs};
use vfs::Vfs;

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    main()
}

fn main() -> ! {
    print("SIGRUN Filesystem Server v0.1\n");

    let mut vfs = Vfs::new();

    let immutable_fs = ImmutableFs::new();

    if vfs.mount("/", immutable_fs).is_err() {
        print("Failed to mount root filesystem\n");
    } else {
        print("Root filesystem mounted successfully\n");
    }

    let mut server = FsServer::new(vfs);

    print("Filesystem server ready, handling requests...\n");

    server.run()
}

fn print(msg: &str) {
    let bytes = msg.as_bytes();
    let args = SyscallArgs::new(4).with_3args(1, bytes.as_ptr() as u64, bytes.len() as u64);
    unsafe {
        syscall_api::syscall(args).ok();
    }
}

pub fn yield_now() {
    let args = SyscallArgs::new(SYSCALL_YIELD);
    unsafe {
        syscall_api::syscall(args).ok();
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    print("PANIC in filesystem\n");
    loop {}
}
