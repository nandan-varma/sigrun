//! SIGRUN Microkernel - Main Entry Point

#![no_std]
#![no_main]
#![allow(unsafe_op_in_unsafe_fn)]
#![feature(abi_x86_interrupt)]

extern crate alloc;

mod arch;
mod capability;
mod error;
mod interrupt;
mod ipc;
mod log;
mod memory;
mod scheduler;
mod timer;

use arch::BootParams;

// Bump allocator backed by a static array - no init needed
use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

const HEAP_SIZE: usize = 4 * 1024 * 1024;

#[repr(align(4096))]
struct StaticHeap([u8; HEAP_SIZE]);

static mut HEAP_MEMORY: StaticHeap = StaticHeap([0; HEAP_SIZE]);
static HEAP_NEXT: AtomicUsize = AtomicUsize::new(0);

struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let base = unsafe { HEAP_MEMORY.0.as_ptr() } as usize;
        loop {
            let current = HEAP_NEXT.load(Ordering::Relaxed);
            let aligned = (current + layout.align() - 1) & !(layout.align() - 1);
            let next = aligned + layout.size();
            if next > HEAP_SIZE {
                return core::ptr::null_mut();
            }
            match HEAP_NEXT.compare_exchange_weak(
                current, next, Ordering::SeqCst, Ordering::Relaxed,
            ) {
                Ok(_) => return (base + aligned) as *mut u8,
                Err(_) => continue,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

unsafe impl Sync for BumpAllocator {}

#[global_allocator]
static GLOBAL_ALLOC: BumpAllocator = BumpAllocator;

/// Entry point called by the multiboot2 boot assembly after entering 64-bit mode.
///
/// Builds a `BootParams` from the multiboot2 information structure and
/// hands off to `kmain`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn kmain_from_multiboot2(magic: u32, info_ptr: u64) -> ! {
    use arch::x86_64::boot::MB2_BOOTLOADER_MAGIC;

    log::early_init();

    if magic != MB2_BOOTLOADER_MAGIC {
        log::error("Bad multiboot2 magic – check bootloader");
        halt();
    }

    extern "C" {
        static __kernel_end: u8;
    }
    let kernel_end = core::ptr::addr_of!(__kernel_end) as u64;
    let kernel_start: u64 = 0x10_0000; // 1 MB physical

    let boot_params = BootParams {
        magic: 0x5349_4752,
        version: 1,
        memory_map: info_ptr as *mut u8,
        memory_map_size: 0,
        memory_descriptor_size: 0,
        kernel_phys_start: kernel_start,
        kernel_virt_start: kernel_start,
        kernel_size: kernel_end.saturating_sub(kernel_start),
        rsdp_address: 0,
        efi_system_table: 0,
    };

    kmain(&boot_params);
}

/// Main kernel entry point called from boot assembly with BootParams in rdi.
#[no_mangle]
pub extern "C" fn kmain(boot_params: &BootParams) -> ! {
    log::early_init();

    log::info("SIGRUN Microkernel v0.1");
    log::info("=======================");

    if !boot_params.validate() {
        log::error("Invalid boot parameters!");
        halt();
    }

    log::info("Boot parameters validated");

    log::info("Initializing memory manager...");
    let mut memory = memory::init(boot_params);
    log::info("Memory manager initialized");

    log::info("Setting up interrupt handling...");
    interrupt::early_init();
    log::info("Interrupt handling initialized");

    log::info("Initializing timer subsystem...");
    timer::init();
    log::info("Timer subsystem initialized");

    log::info("Initializing scheduler...");
    let sched = scheduler::init();
    log::info("Scheduler initialized");

    log::info("Initializing capability manager...");
    let caps = capability::init();
    log::info("Capability manager initialized");

    log::info("Initializing IPC subsystem...");
    ipc::init();
    log::info("IPC subsystem initialized");

    log::info("Creating initial userspace process...");
    let init_process = scheduler::create_init_process();
    log::info("Initial process created");

    log::info("Starting scheduler...");
    scheduler::start();

    halt();
}

fn halt() -> ! {
    loop {
        arch::halt();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    log::error("KERNEL PANIC");
    halt();
}
