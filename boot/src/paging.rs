//! Boot-time Paging Setup
//!
//! This module handles setting up initial page tables for transitioning
//! from UEFI (identity-mapped) to the kernel's expected virtual memory layout.

use crate::memory::MemoryInfo;
use uefi::boot::MemoryType;

/// 4KB page size
const PAGE_SIZE: u64 = 4096;

/// Size of page table entries
const PAGE_TABLE_ENTRY_SIZE: usize = 512;

/// Kernel virtual base address (standard x86_64 mapping)
const KERNEL_VIRT_BASE: u64 = 0xFFFF800000000000;

/// Paging setup errors
#[derive(Debug)]
pub enum PagingError {
    AllocationFailed,
    MappingFailed,
}

/// Set up identity-mapped page tables for early boot
///
/// Creates 4-level paging structure with:
/// 1. Identity mapping for all usable memory (for early boot)
/// 2. Higher-half mapping for kernel (if needed)
pub fn setup_identity_paging(_mem_info: &MemoryInfo) -> Result<(), PagingError> {
    let pml4_phys = allocate_page_table()?;
    let pdpt_phys = allocate_page_table()?;
    let pd_phys = allocate_page_table()?;

    // Initialize tables to zero
    unsafe {
        core::ptr::write_bytes(pml4_phys as *mut u8, 0, PAGE_SIZE as usize);
        core::ptr::write_bytes(pdpt_phys as *mut u8, 0, PAGE_SIZE as usize);
        core::ptr::write_bytes(pd_phys as *mut u8, 0, PAGE_SIZE as usize);
    }

    // Set up PML4 entries
    let pml4 = pml4_phys as *mut u64;
    let pdpt = pdpt_phys as *mut u64;

    unsafe {
        // Entry 0: Identity mapping (low memory)
        *pml4.add(0) = pdpt_phys | PageTableFlags::present_bits();

        // Entry 511: Higher half (kernel space)
        *pml4.add(511) = pdpt_phys | PageTableFlags::present_bits();
    }

    // Set up PDPT entries for identity mapping
    // Map first 512GB of physical memory using 1GB pages
    for i in 0..512 {
        let phys_addr = i as u64 * 1024 * 1024 * 1024; // 1GB pages
        unsafe {
            *pdpt.add(i) = phys_addr | PageTableFlags::huge_page_bits();
        }
    }

    // Load PML4 into CR3
    unsafe {
        load_page_table(pml4_phys);
        enable_paging();
    }

    Ok(())
}

/// Allocate a page-aligned page table
fn allocate_page_table() -> Result<u64, PagingError> {
    let addr = uefi::boot::allocate_pages(
        uefi::boot::AllocateType::AnyPages,
        MemoryType::LOADER_DATA,
        1,
    )
    .map_err(|_| PagingError::AllocationFailed)?;

    Ok(addr.as_ptr() as u64)
}

/// Load a page table address into CR3
unsafe fn load_page_table(_pml4_phys: u64) {
    // Assembly stub - actual paging setup deferred
    // unsafe {
    //     core::arch::asm!(
    //         "mov cr3, {}",
    //         in(reg) pml4_phys,
    //         options(nostack, preserves_flags)
    //     );
    // }
}

/// Enable paging and long mode
unsafe fn enable_paging() {
    // Assembly stubs - actual paging setup deferred
    // unsafe {
    //     // Enable PAE (Physical Address Extension)
    //     core::arch::asm!(
    //         "mov rax, cr4",
    //         "or rax, 0x20", // Set PAE bit
    //         "mov cr4, rax",
    //         options(nostack, preserves_flags)
    //     );
    // }
}

/// Page table entry flags helper
pub struct PageTableFlags;

impl PageTableFlags {
    /// Flags for a present, writable, kernel page table entry
    pub fn present_bits() -> u64 {
        0x3 // Present + Writable
    }

    /// Flags for a 1GB huge page
    pub fn huge_page_bits() -> u64 {
        0x183 // Present + Writable + Huge (1GB)
    }

    /// Check if a flag indicates a present entry
    pub fn is_present(entry: u64) -> bool {
        (entry & 0x1) != 0
    }

    /// Check if a flag indicates a huge page
    pub fn is_huge_page(entry: u64) -> bool {
        (entry & 0x80) != 0
    }
}

/// Create a page table entry from physical address and flags
pub fn make_entry(phys_addr: u64, flags: u64) -> u64 {
    (phys_addr & !0xFFF) | (flags & 0xFFF)
}

/// Get physical address from page table entry
pub fn get_phys_addr(entry: u64) -> u64 {
    entry & !0xFFF
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_page_table_flags() {
        assert!(PageTableFlags::is_present(0x3));
        assert!(!PageTableFlags::is_present(0));
        assert!(PageTableFlags::is_huge_page(0x83));
    }

    #[test]
    fn test_make_entry() {
        let entry = make_entry(0x1000, 0x3);
        assert_eq!(entry, 0x1003);
        assert_eq!(get_phys_addr(entry), 0x1000);
    }
}
