//! x86_64-specific code

pub mod paging;
pub mod gdt;
pub mod idt;

/// Physical address type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PhysAddr(u64);

impl PhysAddr {
    pub fn new(addr: u64) -> Self { Self(addr) }
    pub fn as_u64(self) -> u64 { self.0 }
    pub fn as_ptr<T>(self) -> *const T { self.0 as *const T }
    pub fn as_mut_ptr<T>(self) -> *mut T { self.0 as *mut T }
    
    pub fn page_align(self) -> PhysAddr {
        PhysAddr(self.0 & !0xFFF)
    }
    
    pub fn is_aligned(self, align: u64) -> bool {
        self.0 & (align - 1) == 0
    }
}

/// Virtual address type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VirtAddr(u64);

impl VirtAddr {
    pub fn new(addr: u64) -> Self { Self(addr) }
    pub fn as_u64(self) -> u64 { self.0 }
    pub fn as_ptr<T>(self) -> *const T { self.0 as *const T }
    pub fn as_mut_ptr<T>(self) -> *mut T { self.0 as *mut T }
    
    pub fn page_align(self) -> VirtAddr {
        VirtAddr(self.0 & !0xFFF)
    }
    
    pub fn is_aligned(self, align: u64) -> bool {
        self.0 & (align - 1) == 0
    }
    
    /// Check if address is in higher-half kernel region
    pub fn is_kernel(self) -> bool {
        self.0 >= 0xFFFF_8000_0000_0000
    }
    
    /// Check if address is userspace
    pub fn is_user(self) -> bool {
        self.0 < 0x7FFF_FFFF_FFFF_FFFF
    }
}

/// Page size constants
pub const PAGE_SIZE: u64 = 4096;
pub const PAGE_SHIFT: u64 = 12;
pub const HUGE_PAGE_SIZE: u64 = 2 * 1024 * 1024;
pub const PUD_SIZE: u64 = 512 * 1024 * 1024;
pub const PGD_SIZE: u64 = 512 * 1024 * 1024 * 1024;

/// APIC ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ApicId(u32);

impl ApicId {
    pub fn new(id: u32) -> Self { Self(id) }
    pub fn as_u32(self) -> u32 { self.0 }
}

/// Interrupt vector numbers
pub const IRQ_BASE: u8 = 32;
