//! SIGRUN UEFI Bootloader
//!
//! First-stage bootloader that runs in UEFI environment, loads the kernel,
//! sets up initial paging, and transfers control to the kernel entry point.

#![no_std]
#![no_main]
#![forbid(unsafe_op_in_unsafe_fn)]
// Some paging/params helpers are written ahead of being called from the
// kernel-handoff path that's still being built out.
#![allow(dead_code)]

extern crate alloc;

mod efi;
mod kernel;
mod memory;
mod paging;
mod params;

use params::{BootParams, BOOTINFO_VERSION, SIGRUN_BOOTINFO_MAGIC};
use uefi::allocator::Allocator;
use uefi::prelude::*;

/// Global allocator
#[global_allocator]
static ALLOCATOR: Allocator = Allocator;

/// Panic handler
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    // Try to output error message, but don't panic in panic handler
    uefi::system::with_stdout(|stdout| {
        let _ = stdout.output_string(cstr16!("PANIC in bootloader\r\n"));
    });
    loop {}
}

#[entry]
fn main() -> Status {
    uefi::system::with_stdout(|stdout| {
        let _ = stdout.output_string(cstr16!("SIGRUN Bootloader v0.1\r\n"));
        let _ = stdout.output_string(cstr16!("======================\r\n\r\n"));
    });

    match boot_main() {
        Ok(()) => {
            uefi::system::with_stdout(|stdout| {
                let _ = stdout.output_string(cstr16!("Boot complete. Halting.\r\n"));
            });
            Status::SUCCESS
        }
        Err(e) => {
            let msg = match e {
                BootError::MemoryMap => cstr16!("ERROR: Failed to get memory map\r\n"),
                BootError::KernelLoad => cstr16!("ERROR: Failed to load kernel\r\n"),
                BootError::Paging => cstr16!("ERROR: Failed to setup paging\r\n"),
                BootError::Acpi => cstr16!("ERROR: Failed to find ACPI tables\r\n"),
            };
            uefi::system::with_stdout(|stdout| {
                let _ = stdout.output_string(msg);
            });
            Status::ABORTED
        }
    }
}

#[derive(Debug)]
enum BootError {
    MemoryMap,
    KernelLoad,
    Paging,
    Acpi,
}

fn boot_main() -> Result<(), BootError> {
    uefi::system::with_stdout(|stdout| {
        let _ = stdout.output_string(cstr16!("Phase 1: Parsing memory map...\r\n"));
    });

    let mem_info = memory::get_memory_map().map_err(|_| BootError::MemoryMap)?;

    uefi::system::with_stdout(|stdout| {
        let _ = stdout.output_string(cstr16!("  Memory map parsed\r\n"));
    });

    uefi::system::with_stdout(|stdout| {
        let _ = stdout.output_string(cstr16!("Phase 2: Loading kernel...\r\n"));
    });

    let kernel_info = kernel::load_kernel().map_err(|_| BootError::KernelLoad)?;

    uefi::system::with_stdout(|stdout| {
        let _ = stdout.output_string(cstr16!("  Kernel loaded successfully\r\n"));
    });

    uefi::system::with_stdout(|stdout| {
        let _ = stdout.output_string(cstr16!("Phase 3: Finding ACPI RSDP...\r\n"));
    });

    // Get image handle
    let image_handle = uefi::boot::image_handle();

    // Find RSDP
    let rsdp = efi::find_rsdp().ok_or(BootError::Acpi)?;

    uefi::system::with_stdout(|stdout| {
        let _ = stdout.output_string(cstr16!("  RSDP found\r\n"));
    });

    uefi::system::with_stdout(|stdout| {
        let _ = stdout.output_string(cstr16!("Phase 4: Creating boot parameters...\r\n"));
    });

    let system_table_addr = efi::get_system_table_address();
    let boot_params = create_boot_params(&mem_info, &kernel_info, rsdp, system_table_addr);

    uefi::system::with_stdout(|stdout| {
        let _ = stdout.output_string(cstr16!("Phase 5: Setting up identity paging...\r\n"));
    });

    paging::setup_identity_paging(&mem_info).map_err(|_| BootError::Paging)?;

    uefi::system::with_stdout(|stdout| {
        let _ = stdout.output_string(cstr16!("Phase 6: Exiting boot services...\r\n"));
    });

    // Allocate buffer for memory map before exit
    let mut mmap_buf = [0u8; 4096 * 4];
    let (memory_map, memory_map_size, descriptor_size) =
        memory::exit_boot_services(image_handle, &mut mmap_buf)
            .map_err(|_| BootError::MemoryMap)?;

    uefi::system::with_stdout(|stdout| {
        let _ = stdout.output_string(cstr16!("Jumping to kernel...\r\n"));
    });

    unsafe {
        let entry_ptr = kernel_info.entry_point as *const ();
        let entry: extern "C" fn(&BootParams) -> ! = core::mem::transmute(entry_ptr);

        let mut final_params = boot_params;
        final_params.memory_map = memory_map;
        final_params.memory_map_size = memory_map_size;
        final_params.memory_descriptor_size = descriptor_size;
        final_params.memory_map_entry_count = memory_map_size / descriptor_size;

        entry(&final_params);
    }
}

fn create_boot_params(
    mem_info: &memory::MemoryInfo,
    kernel_info: &kernel::KernelInfo,
    rsdp: u64,
    efi_system_table: u64,
) -> BootParams {
    BootParams {
        magic: SIGRUN_BOOTINFO_MAGIC,
        version: BOOTINFO_VERSION,
        _reserved: 0,
        memory_map: core::ptr::null_mut(),
        memory_map_size: 0,
        memory_descriptor_size: 0,
        memory_map_entry_count: 0,
        kernel_phys_start: kernel_info.phys_start,
        kernel_virt_start: kernel_info.virt_start,
        kernel_size: kernel_info.size,
        rsdp_address: rsdp,
        efi_system_table,
        total_usable_memory: mem_info.total_usable,
        cpu_count: 1,
        boot_flags: 0,
    }
}
