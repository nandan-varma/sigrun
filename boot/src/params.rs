//! Boot Parameters Structure
//!
//! Defines the structure passed from bootloader to kernel at boot time.

/// Magic number for SIGRUN boot info ("SIGR" in ASCII)
pub const SIGRUN_BOOTINFO_MAGIC: u64 = 0x53494752;

/// Current boot info version
pub const BOOTINFO_VERSION: u32 = 1;

/// Boot parameters passed from bootloader to kernel
///
/// This structure is passed to the kernel entry point and contains
/// all information needed to initialize the kernel.
#[repr(C)]
pub struct BootParams {
    /// Magic number (0x53494752 = "SIGR")
    pub magic: u64,

    /// Structure version (currently 1)
    pub version: u32,

    /// Padding for alignment
    pub _reserved: u32,

    /// Pointer to UEFI memory map (physical address)
    pub memory_map: *mut u8,

    /// Size of memory map in bytes
    pub memory_map_size: usize,

    /// Size of each memory descriptor
    pub memory_descriptor_size: usize,

    /// Number of memory map entries
    pub memory_map_entry_count: usize,

    /// Physical address where kernel is loaded
    pub kernel_phys_start: u64,

    /// Virtual address where kernel expects to run
    pub kernel_virt_start: u64,

    /// Size of kernel image in bytes
    pub kernel_size: u64,

    /// Physical address of RSDP (ACPI)
    pub rsdp_address: u64,

    /// EFI System Table physical address
    pub efi_system_table: u64,

    /// Total usable memory in bytes
    pub total_usable_memory: u64,

    /// Number of available CPUs
    pub cpu_count: u32,

    /// Boot flags
    pub boot_flags: u32,
}

impl BootParams {
    /// Validate boot parameters
    ///
    /// Returns true if the boot parameters appear valid.
    pub fn validate(&self) -> bool {
        self.magic == SIGRUN_BOOTINFO_MAGIC
            && self.version == BOOTINFO_VERSION
            && !self.memory_map.is_null()
            && self.kernel_size > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate() {
        let mut mock_mem = [0u8; 1024];
        let params = BootParams {
            magic: SIGRUN_BOOTINFO_MAGIC,
            version: BOOTINFO_VERSION,
            _reserved: 0,
            memory_map: mock_mem.as_mut_ptr(),
            memory_map_size: 1024,
            memory_descriptor_size: 48,
            memory_map_entry_count: 10,
            kernel_phys_start: 0x100000,
            kernel_virt_start: 0xFFFFFFFF80000000,
            kernel_size: 2 * 1024 * 1024,
            rsdp_address: 0,
            efi_system_table: 0,
            total_usable_memory: 1024 * 1024 * 1024,
            cpu_count: 1,
            boot_flags: 0,
        };

        assert!(params.validate());
    }

    #[test]
    fn test_validate_invalid_magic() {
        let mut mock_mem = [0u8; 1024];
        let params = BootParams {
            magic: 0xDEADBEEF,
            version: BOOTINFO_VERSION,
            _reserved: 0,
            memory_map: mock_mem.as_mut_ptr(),
            memory_map_size: 1024,
            memory_descriptor_size: 48,
            memory_map_entry_count: 10,
            kernel_phys_start: 0x100000,
            kernel_virt_start: 0xFFFFFFFF80000000,
            kernel_size: 2 * 1024 * 1024,
            rsdp_address: 0,
            efi_system_table: 0,
            total_usable_memory: 1024 * 1024 * 1024,
            cpu_count: 1,
            boot_flags: 0,
        };

        assert!(!params.validate());
    }
}
