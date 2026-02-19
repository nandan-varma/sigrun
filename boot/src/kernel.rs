//! Kernel Loading from Disk

use super::efi::SystemTable;

/// Kernel information after loading
#[derive(Debug, Clone)]
pub struct KernelInfo {
    pub load_address: u64,
    pub virtual_address: u64,
    pub entry_point: u64,
    pub size: u64,
}

/// Kernel loading errors
#[derive(Debug)]
pub enum LoadError {
    FileNotFound,
    InvalidFormat,
    TooLarge,
    IoError,
}

/// Load kernel from EFI filesystem
pub fn load_kernel(st: &mut SystemTable) -> Result<KernelInfo, LoadError> {
    // In a full implementation:
    // 1. Open EFI file system
    // 2. Read kernel ELF file
    // 3. Parse ELF headers
    // 4. Load segments to memory
    // 5. Apply relocations if needed
    
    // Placeholder: return mock kernel info
    // In real implementation, this would load from disk
    Ok(KernelInfo {
        load_address: 0x100000,  // 1MB - typical kernel load address
        virtual_address: 0xFFFFFFFF80000000,  // Typical x86_64 virtual base
        entry_point: 0x100000,   // Entry point (same as load for simplicity)
        size: 2 * 1024 * 1024,   // 2MB kernel
    })
}

/// Parse ELF header
pub fn parse_elf_header(data: &[u8]) -> Result<ElfHeader, LoadError> {
    // Simplified ELF parser
    if data.len() < 64 {
        return Err(LoadError::InvalidFormat);
    }
    
    // Check ELF magic
    if &data[0..4] != b"\x7fELF" {
        return Err(LoadError::InvalidFormat);
    }
    
    Ok(ElfHeader {
        class: data[4],
        endian: data[5],
        version: data[6],
        phentsize: u16::from_le_bytes([data[42], data[43]]),
        phnum: u16::from_le_bytes([data[44], data[45]]),
        entry: u64::from_le_bytes([data[24..32].try_into().unwrap()]),
    })
}

#[derive(Debug)]
pub struct ElfHeader {
    pub class: u8,
    pub endian: u8,
    pub version: u8,
    pub phentsize: u16,
    pub phnum: u16,
    pub entry: u64,
}
