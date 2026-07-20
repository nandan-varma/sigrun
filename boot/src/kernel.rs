//! Kernel Loading from EFI File System
//!
//! This module handles loading the kernel ELF file from disk,
//! parsing its structure, and preparing it for execution.

use alloc::vec;
use alloc::vec::Vec;
use uefi::boot::MemoryType;
use uefi::cstr16;
use uefi::proto::media::file::{File, FileAttribute, FileMode, RegularFile};
use uefi::proto::media::fs::SimpleFileSystem;

/// Kernel information after loading
#[derive(Debug, Clone)]
pub struct KernelInfo {
    /// Physical address where kernel is loaded
    pub phys_start: u64,

    /// Virtual address where kernel expects to run
    pub virt_start: u64,

    /// Entry point address (physical)
    pub entry_point: u64,

    /// Total size of kernel in memory
    pub size: u64,
}

/// Kernel loading errors
#[derive(Debug)]
pub enum LoadError {
    FileNotFound,
    InvalidFormat,
    TooLarge,
    IoError,
    AllocationFailed,
}

/// Load kernel from EFI filesystem
///
/// This function:
/// 1. Opens the EFI simple filesystem
/// 2. Loads the kernel ELF file
/// 3. Parses ELF headers
/// 4. Loads segments to memory
/// 5. Returns kernel information
pub fn load_kernel() -> Result<KernelInfo, LoadError> {
    let kernel_data = load_kernel_file()?;
    let elf_header = parse_elf_header(&kernel_data)?;

    let (phys_start, virt_start, entry_point, size) = load_elf_segments(&kernel_data, &elf_header)?;

    Ok(KernelInfo {
        phys_start,
        virt_start,
        entry_point,
        size,
    })
}

/// Load the kernel file from disk
fn load_kernel_file() -> Result<Vec<u8>, LoadError> {
    let fs_handle = uefi::boot::get_handle_for_protocol::<SimpleFileSystem>()
        .map_err(|_| LoadError::IoError)?;

    let mut fs = uefi::boot::open_protocol_exclusive::<SimpleFileSystem>(fs_handle)
        .map_err(|_| LoadError::IoError)?;

    let mut root = fs.open_volume().map_err(|_| LoadError::IoError)?;

    let kernel_path = cstr16!("\\EFI\\SIGRUN\\kernel.elf");

    let mut kernel_file = root
        .open(kernel_path, FileMode::Read, FileAttribute::empty())
        .map_err(|_| LoadError::FileNotFound)?;

    let mut file_info_buffer = [0u8; 1024];
    let file_info = kernel_file
        .get_info::<uefi::proto::media::file::FileInfo>(&mut file_info_buffer)
        .map_err(|_| LoadError::IoError)?;

    let file_size = file_info.file_size() as usize;

    if file_size > 64 * 1024 * 1024 {
        return Err(LoadError::TooLarge);
    }

    let mut kernel_data = vec![0u8; file_size];

    let mut kernel_file = unsafe { RegularFile::new(kernel_file) };
    let bytes_read = kernel_file
        .read(&mut kernel_data)
        .map_err(|_| LoadError::IoError)?;

    if bytes_read != file_size {
        return Err(LoadError::IoError);
    }

    Ok(kernel_data)
}

/// ELF header structure
#[derive(Debug, Clone)]
pub struct ElfHeader {
    /// ELF class (1 = 32-bit, 2 = 64-bit)
    pub class: u8,

    /// Endianness (1 = little, 2 = big)
    pub endian: u8,

    /// ELF version
    pub version: u8,

    /// Program header entry size
    pub phentsize: u16,

    /// Number of program headers
    pub phnum: u16,

    /// Entry point address
    pub entry: u64,

    /// Program header offset
    pub phoff: u64,
}

/// Parse ELF header
pub fn parse_elf_header(data: &[u8]) -> Result<ElfHeader, LoadError> {
    if data.len() < 64 {
        return Err(LoadError::InvalidFormat);
    }

    // Check ELF magic number
    if &data[0..4] != b"\x7fELF" {
        return Err(LoadError::InvalidFormat);
    }

    let class = data[4];
    if class != 2 {
        // Only support 64-bit ELF
        return Err(LoadError::InvalidFormat);
    }

    let endian = data[5];
    if endian != 1 {
        // Only support little-endian
        return Err(LoadError::InvalidFormat);
    }

    Ok(ElfHeader {
        class,
        endian,
        version: data[6],
        phentsize: u16::from_le_bytes([data[54], data[55]]),
        phnum: u16::from_le_bytes([data[56], data[57]]),
        entry: u64::from_le_bytes(data[24..32].try_into().unwrap()),
        phoff: u64::from_le_bytes(data[32..40].try_into().unwrap()),
    })
}

/// Load ELF segments into memory
fn load_elf_segments(data: &[u8], header: &ElfHeader) -> Result<(u64, u64, u64, u64), LoadError> {
    let mut min_vaddr = u64::MAX;
    let mut max_vaddr = 0u64;

    let entry_point = header.entry;

    // First pass: determine memory requirements
    for i in 0..header.phnum as usize {
        let ph_offset = header.phoff as usize + i * header.phentsize as usize;

        if ph_offset + 56 > data.len() {
            continue;
        }

        let ph_data = &data[ph_offset..];
        let p_type = u32::from_le_bytes(ph_data[0..4].try_into().unwrap());

        // PT_LOAD = 1
        if p_type != 1 {
            continue;
        }

        let p_vaddr = u64::from_le_bytes(ph_data[16..24].try_into().unwrap());
        let p_memsz = u64::from_le_bytes(ph_data[40..48].try_into().unwrap());

        if p_vaddr < min_vaddr {
            min_vaddr = p_vaddr;
        }
        if p_vaddr + p_memsz > max_vaddr {
            max_vaddr = p_vaddr + p_memsz;
        }
    }

    if min_vaddr == u64::MAX {
        return Err(LoadError::InvalidFormat);
    }

    // Allocate memory for kernel
    let kernel_size = (max_vaddr - min_vaddr).div_ceil(4096) * 4096;

    let kernel_buffer = uefi::boot::allocate_pages(
        uefi::boot::AllocateType::AnyPages,
        MemoryType::LOADER_CODE,
        (kernel_size / 4096) as usize,
    )
    .map_err(|_| LoadError::AllocationFailed)?;

    let kernel_ptr = kernel_buffer.as_ptr();
    let first_phys_addr = kernel_ptr as u64;

    // Clear the buffer
    unsafe {
        core::ptr::write_bytes(kernel_ptr, 0, kernel_size as usize);
    }

    // Second pass: load segments
    for i in 0..header.phnum as usize {
        let ph_offset = header.phoff as usize + i * header.phentsize as usize;
        let ph_data = &data[ph_offset..];
        let p_type = u32::from_le_bytes(ph_data[0..4].try_into().unwrap());

        if p_type != 1 {
            continue;
        }

        let p_offset = u64::from_le_bytes(ph_data[8..16].try_into().unwrap());
        let p_vaddr = u64::from_le_bytes(ph_data[16..24].try_into().unwrap());
        let p_filesz = u64::from_le_bytes(ph_data[32..40].try_into().unwrap());

        let dest_offset = p_vaddr - min_vaddr;
        let src_offset = p_offset as usize;

        if src_offset + p_filesz as usize <= data.len() && p_filesz > 0 {
            unsafe {
                core::ptr::copy_nonoverlapping(
                    data.as_ptr().add(src_offset),
                    kernel_ptr.add(dest_offset as usize),
                    p_filesz as usize,
                );
            }
        }
    }

    Ok((
        first_phys_addr,
        min_vaddr,
        first_phys_addr + (entry_point - min_vaddr),
        kernel_size,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_invalid_elf() {
        let data = [0u8; 64];
        assert!(matches!(
            parse_elf_header(&data),
            Err(LoadError::InvalidFormat)
        ));
    }

    #[test]
    fn test_parse_valid_elf_header() {
        let mut data = vec![0u8; 64];

        // ELF magic
        data[0..4].copy_from_slice(b"\x7fELF");
        // Class: 64-bit
        data[4] = 2;
        // Endian: little
        data[5] = 1;
        // Version
        data[6] = 1;
        // Entry point
        data[24..32].copy_from_slice(&0xFFFFFFFF80001000u64.to_le_bytes());
        // Program header offset
        data[32..40].copy_from_slice(&64u64.to_le_bytes());
        // phentsize
        data[54..56].copy_from_slice(&56u16.to_le_bytes());
        // phnum
        data[56..58].copy_from_slice(&3u16.to_le_bytes());

        let header = parse_elf_header(&data).unwrap();
        assert_eq!(header.class, 2);
        assert_eq!(header.endian, 1);
        assert_eq!(header.entry, 0xFFFFFFFF80001000u64);
        assert_eq!(header.phnum, 3);
    }
}
