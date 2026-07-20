//! Driver Manager Service
//!
//! Manages hardware drivers including PCI enumeration and virtio devices.

#![no_std]
#![no_main]
// This binary is not yet spawned as a real process (see userspace/README /
// CLAUDE.md) — plenty of scaffolding here is written ahead of being wired up.
#![allow(dead_code)]

use core::panic::PanicInfo;
use syscall_api::{SyscallArgs, SYSCALL_WRITE};

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    main()
}

fn main() -> ! {
    print("SIGRUN Driver Manager v0.1\n");
    print("==========================\n\n");

    print("Initializing PCI bus...\n");
    print("Scanning for virtio devices...\n");
    print("Driver Manager ready.\n");

    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}

fn print(s: &str) {
    let args = SyscallArgs::new(SYSCALL_WRITE).with_3args(1, s.as_ptr() as u64, s.len() as u64);
    unsafe {
        let _ = syscall_api::syscall(args);
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    print("PANIC in driver-manager\n");
    loop {}
}
