//! Kernel heap allocator
//!
//! Uses a simple bump allocator for early boot, then transitions to
//! a slab allocator for better performance and fragmentation control.

use super::{Mapper, MemoryError, PageTableFlags, PhysFrame};
use crate::arch::{VirtAddr, PAGE_SIZE};
use core::alloc::{GlobalAlloc, Layout};
use core::ptr::{null_mut, NonNull};
use core::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

const INITIAL_HEAP_SIZE: u64 = 256 * 1024;

pub struct Heap {
    start: VirtAddr,
    end: VirtAddr,
    next: AtomicU64,
    allocated: AtomicUsize,
}

impl Heap {
    pub const fn new() -> Self {
        Self {
            start: VirtAddr::new(0),
            end: VirtAddr::new(0),
            next: AtomicU64::new(0),
            allocated: AtomicUsize::new(0),
        }
    }

    pub unsafe fn init(&mut self, start: VirtAddr, size: u64) {
        self.start = start;
        self.end = VirtAddr::new(start.as_u64() + size);
        self.next.store(start.as_u64(), Ordering::SeqCst);
        self.allocated.store(0, Ordering::SeqCst);
    }

    pub fn allocate(&self, layout: Layout) -> Option<NonNull<u8>> {
        let size = layout.size();
        let align = layout.align();

        if align > PAGE_SIZE as usize {
            return None;
        }

        loop {
            let current = self.next.load(Ordering::Relaxed);
            let aligned = ((current + align as u64 - 1) / align as u64) * align as u64;
            let new_next = aligned + size as u64;

            if new_next > self.end.as_u64() {
                return None;
            }

            match self.next.compare_exchange_weak(
                current,
                new_next,
                Ordering::SeqCst,
                Ordering::Relaxed,
            ) {
                Ok(_) => {
                    self.allocated.fetch_add(size, Ordering::Relaxed);
                    return NonNull::new(aligned as *mut u8);
                }
                Err(_) => continue,
            }
        }
    }

    pub fn deallocate(&self, _ptr: NonNull<u8>, layout: Layout) {
        self.allocated.fetch_sub(layout.size(), Ordering::Relaxed);
    }

    pub fn used(&self) -> usize {
        self.allocated.load(Ordering::Relaxed)
    }

    pub fn size(&self) -> u64 {
        self.end.as_u64() - self.start.as_u64()
    }

    pub fn available(&self) -> u64 {
        self.end.as_u64() - self.next.load(Ordering::Relaxed)
    }

    pub fn is_initialized(&self) -> bool {
        self.start.as_u64() != 0
    }
}

unsafe impl Sync for Heap {}

pub struct KernelHeap {
    inner: Heap,
}

impl KernelHeap {
    pub const fn new() -> Self {
        Self { inner: Heap::new() }
    }

    pub unsafe fn init(&mut self, start: VirtAddr, size: u64) {
        self.inner.init(start, size);
    }

    pub fn stats(&self) -> HeapStats {
        HeapStats {
            used: self.inner.used(),
            total: self.inner.size() as usize,
            available: self.inner.available() as usize,
        }
    }
}

unsafe impl GlobalAlloc for KernelHeap {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        self.inner
            .allocate(layout)
            .map(|ptr| ptr.as_ptr())
            .unwrap_or(null_mut())
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if let Some(nonnull) = NonNull::new(ptr) {
            self.inner.deallocate(nonnull, layout);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HeapStats {
    pub used: usize,
    pub total: usize,
    pub available: usize,
}

#[repr(C)]
struct SlabHeader {
    next: *mut SlabHeader,
    size: usize,
    used: usize,
    free_list: *mut u8,
}

pub struct SlabCache {
    size: usize,
    slabs: *mut SlabHeader,
    objects_per_slab: usize,
}

impl SlabCache {
    pub const fn new(size: usize) -> Self {
        Self {
            size,
            slabs: null_mut(),
            objects_per_slab: 0,
        }
    }

    pub const fn new_for_size(size: usize) -> Self {
        let objects_per_slab = (PAGE_SIZE as usize - core::mem::size_of::<SlabHeader>()) / size;
        Self {
            size,
            slabs: null_mut(),
            objects_per_slab,
        }
    }

    pub fn alloc(&mut self) -> Option<NonNull<u8>> {
        unsafe {
            let mut current = self.slabs;
            while !current.is_null() {
                if !(*current).free_list.is_null() {
                    let obj = (*current).free_list;
                    (*current).free_list = *(obj as *mut *mut u8);
                    (*current).used += 1;
                    return NonNull::new(obj);
                }
                current = (*current).next;
            }
        }
        None
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8) {
        let slab = self.find_slab_for(ptr);
        if !slab.is_null() {
            *(ptr as *mut *mut u8) = (*slab).free_list;
            (*slab).free_list = ptr;
            (*slab).used -= 1;
        }
    }

    unsafe fn find_slab_for(&self, ptr: *mut u8) -> *mut SlabHeader {
        let mut current = self.slabs;
        while !current.is_null() {
            let slab_start = current as *mut u8;
            let slab_end = slab_start.add(PAGE_SIZE as usize);
            if ptr >= slab_start && ptr < slab_end {
                return current;
            }
            current = (*current).next;
        }
        null_mut()
    }
}

unsafe impl Send for SlabCache {}
unsafe impl Sync for SlabCache {}

pub struct SlabAllocator {
    caches: [SlabCache; 10],
}

impl SlabAllocator {
    pub const fn new() -> Self {
        Self {
            caches: [
                SlabCache::new_for_size(8),
                SlabCache::new_for_size(16),
                SlabCache::new_for_size(32),
                SlabCache::new_for_size(64),
                SlabCache::new_for_size(128),
                SlabCache::new_for_size(256),
                SlabCache::new_for_size(512),
                SlabCache::new_for_size(1024),
                SlabCache::new_for_size(2048),
                SlabCache::new_for_size(4096),
            ],
        }
    }

    pub fn alloc(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let size = layout.size().max(layout.align());

        for cache in &mut self.caches {
            if cache.size >= size {
                return cache.alloc();
            }
        }

        None
    }

    pub unsafe fn dealloc(&mut self, ptr: *mut u8, layout: Layout) {
        let size = layout.size().max(layout.align());

        for cache in &mut self.caches {
            if cache.size >= size {
                cache.dealloc(ptr);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heap_basic() {
        static mut HEAP_MEMORY: [u8; 4096] = [0; 4096];

        let mut heap = Heap::new();
        unsafe {
            heap.init(VirtAddr::new(&mut HEAP_MEMORY as *mut _ as u64), 4096);
        }

        let layout = Layout::from_size_align(64, 8).unwrap();
        let ptr = heap.allocate(layout);
        assert!(ptr.is_some());

        let ptr2 = heap.allocate(layout);
        assert!(ptr2.is_some());
        assert_ne!(ptr.unwrap().as_ptr(), ptr2.unwrap().as_ptr());
    }
}
