//! Memory management subsystem
//!
//! This module provides the virtual memory manager, frame allocator,
//! and related memory operations.

extern crate alloc;

use crate::arch::{BootParams, PhysAddr, VirtAddr, PAGE_SIZE};
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU64, Ordering};

pub mod error;
pub mod frame;
pub mod heap;
pub mod mapper;
pub mod page_table;
pub mod region;

pub use error::MemoryError;
pub use frame::{
    order_for_pages, AllocatorStats, BuddyAllocator, FrameAllocator, PhysFrame, ORDER_1GB,
    ORDER_2MB, ORDER_4KB,
};
pub use heap::{Heap, KernelHeap, SlabAllocator, SlabCache};
pub use mapper::{Mapper, PageQuery, PageTableMapper};
pub use page_table::{
    flush_tlb, flush_tlb_all, page_table_indices, read_cr3, write_cr3, Page, PageTable,
    PageTableEntry, PageTableFlags, Pd, Pdpt, Pml4, Pt,
};
pub use region::{
    MemoryMapping, MemoryRegion, RegionList, RegionType, KERNEL_BASE, MMAP_START, USER_BASE,
    VMALLOC_START,
};

static NEXT_ADDRESS_SPACE_ID: AtomicU64 = AtomicU64::new(1);

pub type AddressSpaceId = u64;

#[derive(Debug)]
pub struct AddressSpace {
    pub id: AddressSpaceId,
    pml4_frame: PhysFrame,
    regions: RegionList,
    heap: KernelHeap,
}

impl AddressSpace {
    pub fn new(pml4_frame: PhysFrame) -> Self {
        let id = NEXT_ADDRESS_SPACE_ID.fetch_add(1, Ordering::SeqCst);
        Self {
            id,
            pml4_frame,
            regions: RegionList::new(),
            heap: KernelHeap::new(),
        }
    }

    pub fn kernel_new(pml4_frame: PhysFrame) -> Self {
        Self {
            id: 0,
            pml4_frame,
            regions: RegionList::new(),
            heap: KernelHeap::new(),
        }
    }

    pub fn id(&self) -> AddressSpaceId {
        self.id
    }

    pub fn pml4_frame(&self) -> PhysFrame {
        self.pml4_frame
    }

    pub fn regions(&self) -> &RegionList {
        &self.regions
    }

    pub fn regions_mut(&mut self) -> &mut RegionList {
        &mut self.regions
    }

    pub fn add_region(&mut self, region: MemoryRegion) -> Result<(), MemoryError> {
        self.regions.add(region)
    }

    pub fn remove_region(&mut self, start: VirtAddr) -> Option<MemoryRegion> {
        self.regions.remove(start)
    }

    pub fn find_region(&self, addr: VirtAddr) -> Option<&MemoryRegion> {
        self.regions.find(addr)
    }

    pub unsafe fn init_heap(&mut self, start: VirtAddr, size: u64) {
        self.heap.init(start, size);
    }

    pub fn heap_stats(&self) -> heap::HeapStats {
        self.heap.stats()
    }
}

pub struct MemoryManager {
    frame_allocator: FrameAllocator,
    kernel_space: AddressSpace,
    vmalloc_start: VirtAddr,
    vmalloc_next: AtomicU64,
}

impl MemoryManager {
    pub fn new(kernel_pml4: PhysFrame, _boot_params: &BootParams) -> Result<Self, MemoryError> {
        let mut frame_allocator = FrameAllocator::new();

        let base_addr = PhysAddr::new(0x100000);
        let total_size = 0x10000000u64;

        unsafe {
            frame_allocator.init(base_addr, total_size);
        }

        let kernel_space = AddressSpace::kernel_new(kernel_pml4);

        Ok(Self {
            frame_allocator,
            kernel_space,
            vmalloc_start: VirtAddr::new(VMALLOC_START),
            vmalloc_next: AtomicU64::new(VMALLOC_START),
        })
    }

    pub fn frame_allocator(&mut self) -> &mut FrameAllocator {
        &mut self.frame_allocator
    }

    pub fn kernel_space(&self) -> &AddressSpace {
        &self.kernel_space
    }

    pub fn kernel_space_mut(&mut self) -> &mut AddressSpace {
        &mut self.kernel_space
    }

    pub fn allocate_frame(&mut self, order: usize) -> Result<PhysFrame, MemoryError> {
        self.frame_allocator.allocate(order)
    }

    pub fn allocate_page(&mut self) -> Result<PhysFrame, MemoryError> {
        self.frame_allocator.allocate_page()
    }

    pub fn deallocate_frame(&mut self, frame: PhysFrame) -> Result<(), MemoryError> {
        self.frame_allocator.deallocate(frame)
    }

    pub fn alloc_address_space(&mut self) -> Result<AddressSpace, MemoryError> {
        let pml4_frame = self.allocate_page()?;

        unsafe {
            let pml4 = &mut *(pml4_frame.start.as_mut_ptr::<Pml4>());
            pml4.clear();
        }

        Ok(AddressSpace::new(pml4_frame))
    }

    pub fn free_address_space(&mut self, space: AddressSpace) -> Result<(), MemoryError> {
        self.deallocate_frame(space.pml4_frame)?;
        Ok(())
    }

    pub fn vmalloc(&mut self, size: u64) -> Option<VirtAddr> {
        let aligned_size = ((size + PAGE_SIZE - 1) / PAGE_SIZE) * PAGE_SIZE;

        loop {
            let current = self.vmalloc_next.load(Ordering::Relaxed);
            let new_next = current + aligned_size;

            if new_next > self.vmalloc_start.as_u64() + 0x1000_0000_0000 {
                return None;
            }

            match self.vmalloc_next.compare_exchange_weak(
                current,
                new_next,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Some(VirtAddr::new(current)),
                Err(_) => continue,
            }
        }
    }

    pub fn allocator_stats(&self) -> AllocatorStats {
        self.frame_allocator.stats()
    }
}

unsafe impl Sync for MemoryManager {}

pub fn init(boot_params: &BootParams) -> Result<MemoryManager, MemoryError> {
    crate::log::info_formatted("  - Parsing memory map");

    crate::log::info_formatted("  - Initializing frame allocator");

    let kernel_pml4_phys = unsafe { read_cr3() };
    let kernel_pml4_frame = PhysFrame::new(kernel_pml4_phys, 0);

    crate::log::info_formatted("  - Creating kernel address space");
    let manager = MemoryManager::new(kernel_pml4_frame, boot_params)?;

    crate::log::info_formatted("  - Memory manager initialized");
    Ok(manager)
}

pub fn allocate_page_table() -> Result<&'static mut PageTable, MemoryError> {
    Err(MemoryError::NotMapped)
}

pub struct PhysRegion {
    pub start: PhysAddr,
    pub pages: usize,
}

impl PhysRegion {
    pub fn new(start: PhysAddr, pages: usize) -> Self {
        Self { start, pages }
    }

    pub fn size(&self) -> u64 {
        self.pages as u64 * PAGE_SIZE
    }

    pub fn end(&self) -> PhysAddr {
        PhysAddr::new(self.start.as_u64() + self.size())
    }

    pub fn contains(&self, addr: PhysAddr) -> bool {
        addr.as_u64() >= self.start.as_u64() && addr.as_u64() < self.end().as_u64()
    }
}
