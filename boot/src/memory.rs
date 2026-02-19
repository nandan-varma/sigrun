//! UEFI Memory Map Handling
//!
//! This module handles memory map retrieval, parsing, and management
//! during the boot process.

use alloc::vec::Vec;
use uefi::boot::MemoryType;
use uefi::Status;

/// Memory information collected during boot
#[derive(Debug)]
pub struct MemoryInfo {
    /// Total usable memory in bytes
    pub total_usable: u64,

    /// Memory map entries
    pub entries: Vec<MemoryMapEntry>,

    /// Largest contiguous free region
    pub largest_free_region: Option<MemoryMapEntry>,
}

/// Simplified memory map entry for kernel consumption
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct MemoryMapEntry {
    /// Physical start address
    pub phys_start: u64,

    /// Virtual start address (usually 0 in early boot)
    pub virt_start: u64,

    /// Number of 4KB pages
    pub page_count: u64,

    /// Memory type (UEFI MemoryType)
    pub memory_type: u32,

    /// Memory attributes
    pub attributes: u64,
}

impl MemoryMapEntry {
    /// Size in bytes
    pub fn size(&self) -> u64 {
        self.page_count * 4096
    }

    /// Check if this memory is usable by the kernel
    pub fn is_usable(&self) -> bool {
        // Accept conventional memory type (type 7)
        self.memory_type == 7
    }
}

/// Get the UEFI memory map
pub fn get_memory_map() -> Result<MemoryInfo, Status> {
    let memory_map =
        uefi::boot::memory_map(MemoryType::LOADER_DATA).map_err(|_| Status::OUT_OF_RESOURCES)?;

    use uefi::mem::memory_map::MemoryMap;

    let mut entries = Vec::new();
    let mut total_usable = 0u64;
    let mut largest_free: Option<MemoryMapEntry> = None;
    let mut largest_free_size = 0u64;

    for desc in memory_map.entries() {
        let entry = MemoryMapEntry {
            phys_start: desc.phys_start,
            virt_start: desc.virt_start,
            page_count: desc.page_count,
            memory_type: desc.ty.0,
            attributes: desc.att.bits(),
        };

        if entry.is_usable() {
            total_usable += entry.size();

            if entry.size() > largest_free_size {
                largest_free_size = entry.size();
                largest_free = Some(entry);
            }
        }

        entries.push(entry);
    }

    Ok(MemoryInfo {
        total_usable,
        entries,
        largest_free_region: largest_free,
    })
}

/// Exit boot services and return the final memory map
///
/// This function must be called before jumping to the kernel.
/// It returns the memory map that will be passed to the kernel.
pub fn exit_boot_services(
    _image_handle: uefi::Handle,
    mmap_buf: &mut [u8],
) -> Result<(*mut u8, usize, usize), Status> {
    // Exit boot services with optional memory type
    let _memory_map = unsafe { uefi::boot::exit_boot_services(Some(MemoryType::LOADER_DATA)) };

    // For now, return the buffer we were given
    // The kernel will need to reconstruct the memory map
    let map_size = mmap_buf.len();
    let desc_size = 48; // Typical UEFI memory descriptor size

    Ok((mmap_buf.as_mut_ptr(), map_size, desc_size))
}
