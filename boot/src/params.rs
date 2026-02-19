//! Boot Parameters Structure

/// Magic number for SIGRUN boot info
pub const SIGRUN_BOOTINFO_MAGIC: u64 = 0x53494752; // "SIGR"

/// Current boot info version
pub const BOOTINFO_VERSION: u32 = 1;

/// Boot parameters passed from bootloader to kernel
#[repr(C)]
pub struct BootParams {
    /// Magic number (0x53494752)
    pub magic: u64,
    /// Structure version
    pub version: u32,
    /// Pointer to memory map (physical)
    pub memory_map: *mut u8,
    /// Size of memory map in bytes
    pub memory_map_size: usize,
    /// Size of each memory descriptor
    pub memory_descriptor_size: usize,
    /// Physical address where kernel is loaded
    pub kernel_phys_start: u64,
    /// Virtual address where kernel expects to run
    pub kernel_virt_start: u64,
    /// Size of kernel image in bytes
    pub kernel_size: u64,
    /// Physical address of RSDP
    pub rsdp_address: u64,
    /// EFI System Table physical address
    pub efi_system_table: u64,
}

impl BootParams {
    /// Validate boot parameters
    pub fn validate(&self) -> bool {
        self.magic == SIGRUN_BOOTINFO_MAGIC 
            && self.version == BOOTINFO_VERSION
            && !self.memory_map.is_null()
            && self.kernel_size > 0
    }
}
