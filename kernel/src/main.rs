//! SIGRUN Microkernel - Main Entry Point
//!
//! This is the Rust entry point called from assembly boot code.
//! It receives the BootParams from the bootloader and initializes
//! all kernel subsystems.

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![feature(lang_items, asm_const)]

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

/// Main kernel entry point
///
/// This function is called from the boot assembly with a pointer
/// to the BootParams structure in a register (typically rdi/x0).
#[no_mangle]
pub extern "C" fn kmain(boot_params: &BootParams) -> ! {
    // Initialize early logging
    log::early_init();

    log::info!("SIGRUN Microkernel v0.1");
    log::info!("=======================\n");

    // Validate boot parameters
    if !boot_params.validate() {
        log::error!("Invalid boot parameters!");
        halt();
    }

    log::info!("Boot parameters validated");

    // Phase 1: Early memory initialization
    log::info!("Initializing memory manager...");
    let mut memory = memory::init(boot_params);
    log::info!("Memory manager initialized");

    // Phase 2: Early interrupt setup
    log::info!("Setting up interrupt handling...");
    interrupt::early_init();
    log::info!("Interrupt handling initialized");

    // Phase 3: Timer initialization
    log::info!("Initializing timer subsystem...");
    timer::init();
    log::info!("Timer subsystem initialized");

    // Phase 4: Scheduler initialization
    log::info!("Initializing scheduler...");
    let sched = scheduler::init();
    log::info!("Scheduler initialized");

    // Phase 5: Capability system initialization
    log::info!("Initializing capability manager...");
    let caps = capability::init();
    log::info!("Capability manager initialized");

    // Phase 6: IPC subsystem initialization
    log::info!("Initializing IPC subsystem...");
    ipc::init();
    log::info!("IPC subsystem initialized");

    // Phase 7: Create initial process
    log::info!("Creating initial userspace process...");
    let init_process = scheduler::create_init_process();
    log::info!("Initial process created");

    // Phase 8: Start the scheduler (never returns)
    log::info!("Starting scheduler...\n");
    scheduler::start();

    // Should never reach here
    halt();
}

/// Halt the CPU
fn halt() -> ! {
    loop {
        arch::halt();
    }
}

/// Kernel panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::error(&format!("KERNEL PANIC: {}", info));
    halt();
}

/// Rust runtime stubs
#[lang = "eh_personality"]
fn eh_personality() {}

#[lang = "panic_unwind"]
fn panic_unwind() -> ! {
    halt();
}
