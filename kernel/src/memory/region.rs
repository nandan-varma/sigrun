//! Memory region types for virtual address space management

use super::{PageTableFlags, PhysFrame};
use crate::arch::{PhysAddr, VirtAddr};
use core::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryRegion {
    pub start: VirtAddr,
    pub size: u64,
    pub flags: PageTableFlags,
    pub region_type: RegionType,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionType {
    Code,
    Data,
    Rodata,
    Bss,
    Stack,
    Heap,
    Vmalloc,
    Mmap,
    Device,
    Shared,
    Anonymous,
}

impl MemoryRegion {
    pub fn new(start: VirtAddr, size: u64, flags: PageTableFlags, region_type: RegionType) -> Self {
        Self {
            start,
            size,
            flags,
            region_type,
        }
    }

    pub fn start(&self) -> VirtAddr {
        self.start
    }

    pub fn end(&self) -> VirtAddr {
        VirtAddr::new(self.start.as_u64() + self.size)
    }

    pub fn size(&self) -> u64 {
        self.size
    }

    pub fn contains(&self, addr: VirtAddr) -> bool {
        addr.as_u64() >= self.start.as_u64() && addr.as_u64() < self.end().as_u64()
    }

    pub fn overlaps(&self, other: &MemoryRegion) -> bool {
        self.start.as_u64() < other.end().as_u64() && self.end().as_u64() > other.start.as_u64()
    }
}

impl PartialOrd for MemoryRegion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.start.as_u64().cmp(&other.start.as_u64()))
    }
}

impl Ord for MemoryRegion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.start.as_u64().cmp(&other.start.as_u64())
    }
}

#[derive(Debug, Clone)]
pub struct RegionList {
    regions: [Option<MemoryRegion>; 32],
    count: usize,
}

impl RegionList {
    pub const fn new() -> Self {
        Self {
            regions: [None; 32],
            count: 0,
        }
    }

    pub fn add(&mut self, region: MemoryRegion) -> Result<(), super::error::MemoryError> {
        if self.count >= self.regions.len() {
            return Err(super::error::MemoryError::AddressSpaceFull);
        }

        for i in 0..self.count {
            if let Some(existing) = &self.regions[i] {
                if existing.overlaps(&region) {
                    return Err(super::error::MemoryError::RegionOverlap);
                }
            }
        }

        self.regions[self.count] = Some(region);
        self.count += 1;

        self.sort();

        Ok(())
    }

    pub fn remove(&mut self, addr: VirtAddr) -> Option<MemoryRegion> {
        for i in 0..self.count {
            if let Some(region) = self.regions[i] {
                if region.start() == addr {
                    let removed = self.regions[i].take();
                    self.compact();
                    return removed;
                }
            }
        }
        None
    }

    pub fn find(&self, addr: VirtAddr) -> Option<&MemoryRegion> {
        for i in 0..self.count {
            if let Some(ref region) = self.regions[i] {
                if region.contains(addr) {
                    return Some(region);
                }
            }
        }
        None
    }

    pub fn find_mut(&mut self, addr: VirtAddr) -> Option<&mut MemoryRegion> {
        for i in 0..self.count {
            if let Some(ref mut region) = self.regions[i] {
                if region.contains(addr) {
                    return Some(region);
                }
            }
        }
        None
    }

    fn sort(&mut self) {
        if self.count <= 1 {
            return;
        }

        for i in 0..(self.count - 1) {
            let start_i = match &self.regions[i] {
                Some(r) => r.start.as_u64(),
                None => continue,
            };

            for j in (i + 1)..self.count {
                let start_j = match &self.regions[j] {
                    Some(r) => r.start.as_u64(),
                    None => continue,
                };

                if start_i > start_j {
                    self.regions.swap(i, j);
                }
            }
        }
    }

    fn compact(&mut self) {
        let mut write_idx = 0;
        for read_idx in 0..self.count {
            if self.regions[read_idx].is_some() {
                if write_idx != read_idx {
                    let value = self.regions[read_idx].take();
                    self.regions[write_idx] = value;
                }
                write_idx += 1;
            }
        }
        for i in write_idx..self.count {
            self.regions[i] = None;
        }
        self.count = write_idx;
    }

    pub fn iter(&self) -> impl Iterator<Item = &MemoryRegion> {
        self.regions[..self.count].iter().filter_map(|r| r.as_ref())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut MemoryRegion> {
        self.regions[..self.count]
            .iter_mut()
            .filter_map(|r| r.as_mut())
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn total_size(&self) -> u64 {
        self.iter().map(|r| r.size()).sum()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MemoryMapping {
    pub virt_start: VirtAddr,
    pub phys_start: PhysAddr,
    pub size: u64,
    pub flags: PageTableFlags,
}

impl MemoryMapping {
    pub fn new(
        virt_start: VirtAddr,
        phys_start: PhysAddr,
        size: u64,
        flags: PageTableFlags,
    ) -> Self {
        Self {
            virt_start,
            phys_start,
            size,
            flags,
        }
    }

    pub fn end_virt(&self) -> VirtAddr {
        VirtAddr::new(self.virt_start.as_u64() + self.size)
    }

    pub fn end_phys(&self) -> PhysAddr {
        PhysAddr::new(self.phys_start.as_u64() + self.size)
    }
}

pub const KERNEL_BASE: u64 = 0xFFFF_8000_0000_0000;
pub const USER_BASE: u64 = 0x0000_0000_0040_0000;
pub const VMALLOC_START: u64 = KERNEL_BASE + 0x1000_0000_0000;
pub const MMAP_START: u64 = USER_BASE + 0x100_0000;

pub fn is_kernel_addr(addr: VirtAddr) -> bool {
    addr.as_u64() >= KERNEL_BASE
}

pub fn is_user_addr(addr: VirtAddr) -> bool {
    addr.as_u64() < 0x0000_8000_0000_0000
}
