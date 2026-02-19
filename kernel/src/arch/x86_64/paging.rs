//! x86_64 paging support

use super::{PhysAddr, VirtAddr, PAGE_SIZE};

#[repr(align(4096))]
pub struct PageTable([PageTableEntry; 512]);

impl PageTable {
    pub const fn new() -> Self {
        Self([PageTableEntry::empty(); 512])
    }
}

#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub fn new(frame: PhysAddr, flags: PageTableFlags) -> Self {
        Self(frame.as_u64() | flags.bits())
    }

    pub fn is_present(&self) -> bool {
        self.0 & 1 != 0
    }

    pub fn frame(&self) -> Option<PhysAddr> {
        if self.is_present() {
            Some(PhysAddr::new(self.0 & 0x000F_FFFF_FFFF_F000))
        } else {
            None
        }
    }

    pub fn flags(&self) -> PageTableFlags {
        PageTableFlags::from_bits_truncate(self.0)
    }
}

bitflags::bitflags! {
    pub struct PageTableFlags: u64 {
        const PRESENT = 1;
        const WRITABLE = 1 << 1;
        const USER_ACCESSIBLE = 1 << 2;
        const WRITE_THROUGH = 1 << 3;
        const NO_CACHE = 1 << 4;
        const ACCESSED = 1 << 5;
        const DIRTY = 1 << 6;
        const HUGE_PAGE = 1 << 7;
        const GLOBAL = 1 << 8;
        const NO_EXECUTE = 1 << 63;
    }
}

pub const fn virt_to_page(virt: VirtAddr) -> u64 {
    virt.as_u64() / PAGE_SIZE
}

/// Read CR2 register (faulting address for page faults)
pub fn get_cr2() -> u64 {
    let cr2: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr2",
            out(reg) cr2,
            options(nostack, preserves_flags)
        );
    }
    cr2
}

/// Read CR3 register (current page table base)
pub fn get_cr3() -> u64 {
    let cr3: u64;
    unsafe {
        core::arch::asm!(
            "mov {}, cr3",
            out(reg) cr3,
            options(nostack, preserves_flags)
        );
    }
    cr3
}
