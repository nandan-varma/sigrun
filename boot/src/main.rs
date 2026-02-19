//! SIGRUN UEFI Bootloader
//!
//! This is the first stage bootloader that runs in UEFI environment,
//! loads the kernel into memory, sets up initial paging, and transfers
//! control to the kernel entry point.

#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]
#![feature(abi_efiapi)]

mod efi;
mod memory;
mod kernel;
mod paging;
mod params;

use core::ptr;
use efi::{SystemTable, Status};

/// UEFI application entry point
#[entry]
pub fn main(image_handle: efi::Handle, system_table: *mut SystemTable) -> Status {
    let st = unsafe { &mut *system_table };
    
    // Initialize console for early debug output
    let stdout = st.stdout();
    let _ = stdout.write_str("SIGRUN Bootloader v0.1\r\n");
    let _ = stdout.write_str("======================\r\n\r\n");
    
    // Phase 1: Parse UEFI memory map
    let mem_map = match memory::parse_memory_map(st) {
        Ok(m) => {
            let _ = stdout.write_str(&format!("Memory map: {} entries\r\n", m.len()));
            m
        }
        Err(e) => {
            let _ = stdout.write_str(&format!("Failed to parse memory map: {:?}\r\n", e));
            return Status::LOAD_ERROR;
        }
    };
    
    // Phase 2: Load kernel from disk
    let kernel_info = match kernel::load_kernel(st) {
        Ok(k) => {
            let _ = stdout.write_str(&format!(
                "Kernel loaded: {}KB at {:#018x}\r\n",
                k.size / 1024, k.load_address
            ));
            k
        }
        Err(e) => {
            let _ = stdout.write_str(&format!("Failed to load kernel: {:?}\r\n", e));
            return Status::LOAD_ERROR;
        }
    };
    
    // Phase 3: Find RSDP for OS
    let rsdp = efi::find_rsdp(st);
    let _ = stdout.write_str(&format!("RSDP: {:#018x}\r\n", rsdp));
    
    // Phase 4: Create boot parameters
    let boot_params = params::BootParams {
        magic: params::SIGRUN_BOOTINFO_MAGIC,
        version: params::BOOTINFO_VERSION,
        memory_map: ptr::null_mut(),
        memory_map_size: 0,
        memory_descriptor_size: 0,
        kernel_phys_start: kernel_info.load_address,
        kernel_virt_start: kernel_info.virtual_address,
        kernel_size: kernel_info.size,
        rsdp_address: rsdp,
        efi_system_table: system_table as u64,
    };
    
    // Phase 5: Set up boot-time paging
    paging::setup_identity_paging();
    
    // Phase 6: Jump to kernel
    let _ = stdout.write_str("\r\nBooting SIGRUN kernel...\r\n");
    
    unsafe {
        let entry: extern "C" fn(&params::BootParams) -> ! = 
            core::mem::transmute(kernel_info.entry_point);
        entry(&boot_params);
    }
    
    // Should never return
    Status::ABORTED
}
