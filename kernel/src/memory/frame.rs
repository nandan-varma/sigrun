//! Physical frame allocator using buddy system
//!
//! Supports allocations of 4KB, 8KB, 16KB, ..., up to 4GB blocks

use super::error::MemoryError;
use crate::arch::{PhysAddr, PAGE_SIZE};
use core::sync::atomic::{AtomicUsize, Ordering};

pub const MAX_ORDER: usize = 20;
pub const MIN_ORDER: usize = 0;
pub const ORDER_4KB: usize = 0;
pub const ORDER_2MB: usize = 9;
pub const ORDER_1GB: usize = 18;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysFrame {
    pub start: PhysAddr,
    pub order: usize,
}

impl PhysFrame {
    pub fn new(start: PhysAddr, order: usize) -> Self {
        Self { start, order }
    }

    pub fn size(&self) -> u64 {
        PAGE_SIZE << self.order
    }

    pub fn start_address(&self) -> PhysAddr {
        self.start
    }

    pub fn end_address(&self) -> PhysAddr {
        PhysAddr::new(self.start.as_u64() + self.size())
    }

    pub fn pfn(&self) -> u64 {
        self.start.as_u64() / PAGE_SIZE
    }

    pub fn contains(&self, addr: PhysAddr) -> bool {
        addr.as_u64() >= self.start.as_u64() && addr.as_u64() < self.end_address().as_u64()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AllocatorStats {
    pub total_frames: usize,
    pub used_frames: usize,
    pub free_frames: usize,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
}

struct FreeList {
    head: Option<*mut FreeBlock>,
    count: usize,
}

struct FreeBlock {
    next: Option<*mut FreeBlock>,
}

impl FreeList {
    const fn new() -> Self {
        Self {
            head: None,
            count: 0,
        }
    }

    fn push(&mut self, block: *mut FreeBlock) {
        unsafe {
            (*block).next = self.head;
        }
        self.head = Some(block);
        self.count += 1;
    }

    fn pop(&mut self) -> Option<*mut FreeBlock> {
        let block = self.head?;
        unsafe {
            self.head = (*block).next;
        }
        self.count -= 1;
        Some(block)
    }

    fn is_empty(&self) -> bool {
        self.head.is_none()
    }
}

pub struct BuddyAllocator {
    base: PhysAddr,
    total_frames: usize,
    free_lists: [FreeList; MAX_ORDER + 1],
    used_frames: AtomicUsize,
    initialized: bool,
}

impl BuddyAllocator {
    pub const fn new() -> Self {
        Self {
            base: PhysAddr::new(0),
            total_frames: 0,
            free_lists: [
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
                FreeList::new(),
            ],
            used_frames: AtomicUsize::new(0),
            initialized: false,
        }
    }

    pub unsafe fn init(&mut self, base: PhysAddr, size: u64) {
        self.base = base;
        self.total_frames = (size / PAGE_SIZE) as usize;
        self.initialized = true;

        let mut remaining = size;
        let mut current = base;

        while remaining >= PAGE_SIZE {
            let mut order = MAX_ORDER;
            while order > 0 {
                let block_size = PAGE_SIZE << order;
                if remaining >= block_size && current.as_u64() % block_size == 0 {
                    break;
                }
                order -= 1;
            }

            let block_size = PAGE_SIZE << order;
            self.add_free_block(current, order);
            current = PhysAddr::new(current.as_u64() + block_size);
            remaining -= block_size;
        }
    }

    unsafe fn add_free_block(&mut self, addr: PhysAddr, order: usize) {
        let block = addr.as_mut_ptr::<FreeBlock>();
        self.free_lists[order].push(block);
    }

    pub fn allocate(&mut self, order: usize) -> Result<PhysFrame, MemoryError> {
        if order > MAX_ORDER {
            return Err(MemoryError::InvalidOrder);
        }

        let mut actual_order = order;
        while actual_order <= MAX_ORDER && self.free_lists[actual_order].is_empty() {
            actual_order += 1;
        }

        if actual_order > MAX_ORDER {
            return Err(MemoryError::OutOfFrames);
        }

        let block = self.free_lists[actual_order].pop().unwrap();
        let addr = PhysAddr::new(block as u64);

        while actual_order > order {
            actual_order -= 1;
            let buddy_addr = PhysAddr::new(addr.as_u64() + (PAGE_SIZE << actual_order));
            unsafe {
                self.add_free_block(buddy_addr, actual_order);
            }
        }

        let frames_in_block = 1 << order;
        self.used_frames
            .fetch_add(frames_in_block, Ordering::Relaxed);

        Ok(PhysFrame::new(addr, order))
    }

    pub fn allocate_sized(&mut self, size: u64) -> Result<PhysFrame, MemoryError> {
        if size == 0 {
            return Err(MemoryError::InvalidSize);
        }

        let pages_needed = (size + PAGE_SIZE - 1) / PAGE_SIZE;
        let order = order_for_pages(pages_needed as usize);
        self.allocate(order)
    }

    pub fn deallocate(&mut self, frame: PhysFrame) -> Result<(), MemoryError> {
        let order = frame.order;
        let mut addr = frame.start;

        self.used_frames.fetch_sub(1 << order, Ordering::Relaxed);

        for current_order in order..MAX_ORDER {
            let buddy_addr = self.buddy_address(addr, current_order);

            unsafe {
                if !self.is_free(buddy_addr, current_order) {
                    break;
                }

                self.remove_from_free_list(buddy_addr, current_order);
            }

            if buddy_addr.as_u64() < addr.as_u64() {
                addr = buddy_addr;
            }
        }

        unsafe {
            self.add_free_block(addr, order);
        }

        Ok(())
    }

    fn buddy_address(&self, addr: PhysAddr, order: usize) -> PhysAddr {
        let block_size = PAGE_SIZE << order;
        PhysAddr::new((addr.as_u64() ^ block_size))
    }

    unsafe fn is_free(&self, addr: PhysAddr, order: usize) -> bool {
        if addr.as_u64() < self.base.as_u64() {
            return false;
        }

        let max_addr = self.base.as_u64() + (self.total_frames as u64 * PAGE_SIZE);
        if addr.as_u64() >= max_addr {
            return false;
        }

        let mut current = self.free_lists[order].head;
        while let Some(block) = current {
            if block as u64 == addr.as_u64() {
                return true;
            }
            current = (*block).next;
        }
        false
    }

    unsafe fn remove_from_free_list(&mut self, addr: PhysAddr, order: usize) {
        let target = addr.as_u64() as *mut FreeBlock;
        let mut current = self.free_lists[order].head;
        let mut prev: Option<*mut FreeBlock> = None;

        while let Some(block) = current {
            if block == target {
                if let Some(p) = prev {
                    (*p).next = (*block).next;
                } else {
                    self.free_lists[order].head = (*block).next;
                }
                self.free_lists[order].count -= 1;
                return;
            }
            prev = Some(block);
            current = (*block).next;
        }
    }

    pub fn stats(&self) -> AllocatorStats {
        let used = self.used_frames.load(Ordering::Relaxed);
        let free = self.total_frames.saturating_sub(used);

        AllocatorStats {
            total_frames: self.total_frames,
            used_frames: used,
            free_frames: free,
            total_bytes: self.total_frames as u64 * PAGE_SIZE,
            used_bytes: used as u64 * PAGE_SIZE,
            free_bytes: free as u64 * PAGE_SIZE,
        }
    }

    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
}

pub fn order_for_pages(pages: usize) -> usize {
    if pages == 0 {
        return 0;
    }
    let mut order = 0;
    let mut size = 1;
    while size < pages {
        order += 1;
        size <<= 1;
    }
    order.min(MAX_ORDER)
}

pub struct FrameAllocator {
    buddy: BuddyAllocator,
    reserved_regions: [(PhysAddr, PhysAddr); 16],
    reserved_count: usize,
}

impl FrameAllocator {
    pub const fn new() -> Self {
        Self {
            buddy: BuddyAllocator::new(),
            reserved_regions: [(PhysAddr::new(0), PhysAddr::new(0)); 16],
            reserved_count: 0,
        }
    }

    pub unsafe fn init(&mut self, base: PhysAddr, size: u64) {
        self.buddy.init(base, size);
    }

    pub fn allocate(&mut self, order: usize) -> Result<PhysFrame, MemoryError> {
        self.buddy.allocate(order)
    }

    pub fn allocate_sized(&mut self, size: u64) -> Result<PhysFrame, MemoryError> {
        self.buddy.allocate_sized(size)
    }

    pub fn allocate_page(&mut self) -> Result<PhysFrame, MemoryError> {
        self.allocate(ORDER_4KB)
    }

    pub fn allocate_2mb(&mut self) -> Result<PhysFrame, MemoryError> {
        self.allocate(ORDER_2MB)
    }

    pub fn deallocate(&mut self, frame: PhysFrame) -> Result<(), MemoryError> {
        self.buddy.deallocate(frame)
    }

    pub fn add_reserved(&mut self, start: PhysAddr, end: PhysAddr) {
        if self.reserved_count < self.reserved_regions.len() {
            self.reserved_regions[self.reserved_count] = (start, end);
            self.reserved_count += 1;
        }
    }

    pub fn is_reserved(&self, addr: PhysAddr) -> bool {
        for i in 0..self.reserved_count {
            let (start, end) = self.reserved_regions[i];
            if addr.as_u64() >= start.as_u64() && addr.as_u64() < end.as_u64() {
                return true;
            }
        }
        false
    }

    pub fn stats(&self) -> AllocatorStats {
        self.buddy.stats()
    }
}
