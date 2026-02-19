//! x86_64 page table structures
//!
//! Implements 4-level paging (PML4, PDPT, PD, PT)

use super::{MemoryError, PhysFrame};
use crate::arch::{PhysAddr, VirtAddr, PAGE_SIZE};
use bitflags::bitflags;

pub const PAGE_TABLE_ENTRIES: usize = 512;
pub const PAGE_TABLE_SIZE: u64 = PAGE_SIZE;

pub const PML4_SHIFT: u64 = 39;
pub const PDPT_SHIFT: u64 = 30;
pub const PD_SHIFT: u64 = 21;
pub const PT_SHIFT: u64 = 12;

pub const PML4_MASK: u64 = 0o777 << PML4_SHIFT;
pub const PDPT_MASK: u64 = 0o777 << PDPT_SHIFT;
pub const PD_MASK: u64 = 0o777 << PD_SHIFT;
pub const PT_MASK: u64 = 0o777 << PT_SHIFT;

bitflags! {
    #[repr(transparent)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PageTableFlags: u64 {
        const PRESENT = 1 << 0;
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

impl PageTableFlags {
    pub const fn kernel_code() -> Self {
        Self::PRESENT.union(Self::WRITABLE)
    }

    pub const fn kernel_data() -> Self {
        Self::PRESENT.union(Self::WRITABLE)
    }

    pub const fn kernel_rodata() -> Self {
        Self::PRESENT
    }

    pub const fn user_code() -> Self {
        Self::PRESENT
            .union(Self::WRITABLE)
            .union(Self::USER_ACCESSIBLE)
    }

    pub const fn user_data() -> Self {
        Self::PRESENT
            .union(Self::WRITABLE)
            .union(Self::USER_ACCESSIBLE)
    }

    pub const fn user_rodata() -> Self {
        Self::PRESENT.union(Self::USER_ACCESSIBLE)
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct PageTableEntry(u64);

impl PageTableEntry {
    pub const fn empty() -> Self {
        Self(0)
    }

    pub fn new(frame: PhysFrame, flags: PageTableFlags) -> Self {
        assert!(
            frame.order == 0,
            "Only 4KB frames can be used in page tables"
        );
        Self(frame.start.as_u64() | flags.bits())
    }

    pub fn is_unused(&self) -> bool {
        self.0 == 0
    }

    pub fn is_present(&self) -> bool {
        self.0 & PageTableFlags::PRESENT.bits() != 0
    }

    pub fn is_huge(&self) -> bool {
        self.0 & PageTableFlags::HUGE_PAGE.bits() != 0
    }

    pub fn flags(&self) -> PageTableFlags {
        PageTableFlags::from_bits_truncate(self.0)
    }

    pub fn frame(&self) -> PhysFrame {
        PhysFrame::new(PhysAddr::new(self.0 & !0xFFF), 0)
    }

    pub fn addr(&self) -> PhysAddr {
        PhysAddr::new(self.0 & !0xFFF)
    }

    pub fn set(&mut self, frame: PhysFrame, flags: PageTableFlags) {
        assert!(
            frame.order == 0,
            "Only 4KB frames can be used in page tables"
        );
        self.0 = frame.start.as_u64() | flags.bits();
    }

    pub fn clear(&mut self) {
        self.0 = 0;
    }

    pub fn as_u64(&self) -> u64 {
        self.0
    }
}

#[repr(align(4096))]
pub struct PageTable {
    pub entries: [PageTableEntry; PAGE_TABLE_ENTRIES],
}

impl PageTable {
    pub const fn new() -> Self {
        Self {
            entries: [PageTableEntry::empty(); PAGE_TABLE_ENTRIES],
        }
    }

    pub fn as_ptr(&self) -> *const PageTable {
        self as *const Self
    }

    pub fn as_mut_ptr(&mut self) -> *mut PageTable {
        self as *mut Self
    }

    pub fn as_phys_addr(&self) -> PhysAddr {
        PhysAddr::new(self.as_ptr() as u64)
    }

    pub fn get(&self, index: usize) -> Option<&PageTableEntry> {
        self.entries.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut PageTableEntry> {
        self.entries.get_mut(index)
    }

    pub fn iter(&self) -> impl Iterator<Item = &PageTableEntry> {
        self.entries.iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut PageTableEntry> {
        self.entries.iter_mut()
    }

    pub fn clear(&mut self) {
        for entry in &mut self.entries {
            entry.clear();
        }
    }
}

pub type Pml4 = PageTable;
pub type Pdpt = PageTable;
pub type Pd = PageTable;
pub type Pt = PageTable;

pub fn page_table_indices(addr: VirtAddr) -> (usize, usize, usize, usize) {
    let addr_val = addr.as_u64();
    let pml4_idx = ((addr_val >> PML4_SHIFT) & 0o777) as usize;
    let pdpt_idx = ((addr_val >> PDPT_SHIFT) & 0o777) as usize;
    let pd_idx = ((addr_val >> PD_SHIFT) & 0o777) as usize;
    let pt_idx = ((addr_val >> PT_SHIFT) & 0o777) as usize;
    (pml4_idx, pdpt_idx, pd_idx, pt_idx)
}

pub fn page_table_addr(pml4_idx: usize, pdpt_idx: usize, pd_idx: usize, pt_idx: usize) -> VirtAddr {
    let addr = (pml4_idx as u64) << PML4_SHIFT
        | (pdpt_idx as u64) << PDPT_SHIFT
        | (pd_idx as u64) << PD_SHIFT
        | (pt_idx as u64) << PT_SHIFT;
    VirtAddr::new(addr)
}

#[derive(Debug, Clone, Copy)]
pub struct Page {
    addr: VirtAddr,
}

impl Page {
    pub fn from_addr(addr: VirtAddr) -> Self {
        Self {
            addr: addr.page_align(),
        }
    }

    pub fn from_index(index: u64) -> Self {
        Self {
            addr: VirtAddr::new(index * PAGE_SIZE),
        }
    }

    pub fn addr(&self) -> VirtAddr {
        self.addr
    }

    pub fn p4_index(&self) -> usize {
        ((self.addr.as_u64() >> PML4_SHIFT) & 0o777) as usize
    }

    pub fn p3_index(&self) -> usize {
        ((self.addr.as_u64() >> PDPT_SHIFT) & 0o777) as usize
    }

    pub fn p2_index(&self) -> usize {
        ((self.addr.as_u64() >> PD_SHIFT) & 0o777) as usize
    }

    pub fn p1_index(&self) -> usize {
        ((self.addr.as_u64() >> PT_SHIFT) & 0o777) as usize
    }
}

pub fn page_table_frame(page_table: &PageTable) -> PhysFrame {
    PhysFrame::new(PhysAddr::new(page_table as *const PageTable as u64), 0)
}

pub unsafe fn read_cr3() -> PhysAddr {
    let addr: u64;
    core::arch::asm!(
        "mov {}, cr3",
        out(reg) addr,
        options(nomem, nostack)
    );
    PhysAddr::new(addr)
}

pub unsafe fn write_cr3(addr: PhysAddr) {
    core::arch::asm!(
        "mov cr3, {}",
        in(reg) addr.as_u64(),
        options(nomem, nostack)
    );
}

pub fn flush_tlb(addr: VirtAddr) {
    unsafe {
        core::arch::asm!(
            "invlpg [{}]",
            in(reg) addr.as_u64(),
            options(nostack)
        );
    }
}

pub fn flush_tlb_all() {
    let old_cr3 = unsafe { read_cr3() };
    unsafe { write_cr3(old_cr3) };
}
