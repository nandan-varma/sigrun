//! UEFI Memory Map Parsing

use super::efi::{SystemTable, MemoryDescriptor, MEMORY_TYPE_CONVENTIONAL};

/// Memory map entry for kernel
#[derive(Debug, Clone)]
pub struct MemoryRegion {
    pub phys_start: u64,
    pub virt_start: u64,
    pub page_count: u64,
    pub memory_type: u32,
    pub usable: bool,
}

/// Parse UEFI memory map
pub fn parse_memory_map(st: &mut SystemTable) -> Result<Vec<MemoryRegion>, ()> {
    // In a full implementation, this would:
    // 1. Call GetMemoryMap boot service
    // 2. Allocate buffer for memory map
    // 3. Parse each MemoryDescriptor
    
    // For now, return placeholder that would be filled in real implementation
    Ok(vec![
        MemoryRegion {
            phys_start: 0x0,
            virt_start: 0x0,
            page_count: 0x100, // 1MB
            memory_type: MEMORY_TYPE_CONVENTIONAL,
            usable: true,
        },
    ])
}

/// Calculate total usable memory from memory map
pub fn calculate_usable_memory(regions: &[MemoryRegion]) -> u64 {
    regions
        .iter()
        .filter(|r| r.usable)
        .map(|r| r.page_count * 4096)
        .sum()
}
