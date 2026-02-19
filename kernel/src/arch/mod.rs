//! Architecture-specific code for SIGRUN kernel

pub mod x86_64;

pub use x86_64::{ApicId, PhysAddr, VirtAddr, PAGE_SIZE};

/// Boot parameters from bootloader
#[repr(C)]
pub struct BootParams {
    pub magic: u64,
    pub version: u32,
    pub memory_map: *mut u8,
    pub memory_map_size: usize,
    pub memory_descriptor_size: usize,
    pub kernel_phys_start: u64,
    pub kernel_virt_start: u64,
    pub kernel_size: u64,
    pub rsdp_address: u64,
    pub efi_system_table: u64,
}

impl BootParams {
    pub fn validate(&self) -> bool {
        const SIGRUN_MAGIC: u64 = 0x53494752;
        self.magic == SIGRUN_MAGIC && self.version == 1 && self.kernel_size > 0
    }
}

/// Halt the CPU
pub fn halt() -> ! {
    #[cfg(target_arch = "x86_64")]
    {
        use core::arch::asm;
        unsafe {
            asm!("hlt", options(nomem, nostack));
        }
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        use core::arch::asm;
        unsafe {
            asm!("wfi", options(nomem, nostack));
        }
    }
    loop {}
}

/// Enable interrupts
pub fn enable_interrupts() {
    #[cfg(target_arch = "x86_64")]
    {
        use core::arch::asm;
        unsafe {
            asm!("sti", options(nomem, nostack));
        }
    }
}

/// Disable interrupts
pub fn disable_interrupts() {
    #[cfg(target_arch = "x86_64")]
    {
        use core::arch::asm;
        unsafe {
            asm!("cli", options(nomem, nostack));
        }
    }
}

/// Read the current flags register
pub fn read_flags() -> u64 {
    #[cfg(target_arch = "x86_64")]
    {
        use core::arch::asm;
        let flags: u64;
        unsafe {
            asm!("pushfq; pop {}", out(reg) flags, options(nomem));
        }
        flags
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        0
    }
}

/// CPU ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuId(pub u32);

/// Get current CPU ID
pub fn get_cpu_id() -> CpuId {
    #[cfg(target_arch = "x86_64")]
    {
        use core::arch::asm;
        let id: u32;
        unsafe {
            asm!("cpuid", out("eax") id, options(nomem, nostack));
        }
        CpuId(id)
    }
    #[cfg(not(target_arch = "x86_64"))]
    {
        CpuId(0)
    }
}
