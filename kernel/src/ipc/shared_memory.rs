//! Shared Memory IPC Fast Path
//!
//! Provides zero-copy data transfer through shared memory regions.
//! Used for large data transfers between processes.

extern crate alloc;

use alloc::collections::BTreeMap;
use alloc::sync::Arc;
use alloc::vec::Vec;
use core::sync::atomic::{AtomicU32, AtomicU64, Ordering};

use super::endpoint::ProcessId;

static SHM_REGION_COUNTER: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShmId(u64);

impl ShmId {
    pub fn new() -> Self {
        Self(SHM_REGION_COUNTER.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhysFrame(u64);

impl PhysFrame {
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    pub fn addr(self) -> u64 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtAddr(u64);

impl VirtAddr {
    pub const fn new(addr: u64) -> Self {
        Self(addr)
    }

    pub fn addr(self) -> u64 {
        self.0
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MemoryRights: u32 {
        const NONE = 0;
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXECUTE = 1 << 2;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShareMode {
    CopyOnWrite,
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone)]
pub struct SharedMemoryRegion {
    pub id: ShmId,
    pub frames: Vec<PhysFrame>,
    pub page_count: usize,
    pub rights: MemoryRights,
    pub share_mode: ShareMode,
    pub owner: ProcessId,
    ref_count: AtomicU32,
}

impl SharedMemoryRegion {
    pub fn new(
        frames: Vec<PhysFrame>,
        rights: MemoryRights,
        share_mode: ShareMode,
        owner: ProcessId,
    ) -> Self {
        let page_count = frames.len();
        Self {
            id: ShmId::new(),
            frames,
            page_count,
            rights,
            share_mode,
            owner,
            ref_count: AtomicU32::new(1),
        }
    }

    pub fn with_size(page_count: usize, owner: ProcessId) -> Self {
        let frames: Vec<PhysFrame> = (0..page_count)
            .map(|i| PhysFrame::new((i * 4096) as u64))
            .collect();

        Self::new(
            frames,
            MemoryRights::READ | MemoryRights::WRITE,
            ShareMode::ReadWrite,
            owner,
        )
    }

    pub fn size_bytes(&self) -> usize {
        self.page_count * 4096
    }

    pub fn inc_ref(&self) -> u32 {
        self.ref_count.fetch_add(1, Ordering::AcqRel) + 1
    }

    pub fn dec_ref(&self) -> u32 {
        self.ref_count.fetch_sub(1, Ordering::AcqRel) - 1
    }

    pub fn ref_count(&self) -> u32 {
        self.ref_count.load(Ordering::Acquire)
    }

    pub fn derive(&self, rights: MemoryRights, mode: ShareMode) -> Self {
        let new_rights = self.rights & rights;
        self.inc_ref();

        Self {
            id: ShmId::new(),
            frames: self.frames.clone(),
            page_count: self.page_count,
            rights: new_rights,
            share_mode: mode,
            owner: self.owner,
            ref_count: AtomicU32::new(1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShmHandle(u64);

impl ShmHandle {
    pub fn new(id: u64) -> Self {
        Self(id)
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
}

#[derive(Debug)]
pub struct ShmMapping {
    pub handle: ShmHandle,
    pub region_id: ShmId,
    pub vaddr: VirtAddr,
    pub size: usize,
    pub process: ProcessId,
}

#[derive(Debug)]
pub enum ShmError {
    RegionNotFound,
    MappingFailed,
    InvalidHandle,
    PermissionDenied,
    OutOfMemory,
}

pub struct SharedMemoryManager {
    regions: spin::RwLock<BTreeMap<ShmId, Arc<SharedMemoryRegion>>>,
    mappings: spin::RwLock<BTreeMap<ShmHandle, ShmMapping>>,
    handle_counter: AtomicU64,
}

impl SharedMemoryManager {
    pub const fn new() -> Self {
        Self {
            regions: spin::RwLock::new(BTreeMap::new()),
            mappings: spin::RwLock::new(BTreeMap::new()),
            handle_counter: AtomicU64::new(1),
        }
    }

    pub fn create_region(
        &self,
        page_count: usize,
        rights: MemoryRights,
        mode: ShareMode,
        owner: ProcessId,
    ) -> Result<Arc<SharedMemoryRegion>, ShmError> {
        let region = SharedMemoryRegion::with_size(page_count, owner);
        region.rights = rights;
        region.share_mode = mode;

        let arc_region = Arc::new(region);
        let id = arc_region.id;
        self.regions.write().insert(id, arc_region.clone());

        Ok(arc_region)
    }

    pub fn get_region(&self, id: ShmId) -> Option<Arc<SharedMemoryRegion>> {
        self.regions.read().get(&id).cloned()
    }

    pub fn map_region(
        &self,
        region: &SharedMemoryRegion,
        process: ProcessId,
        vaddr: VirtAddr,
    ) -> Result<ShmHandle, ShmError> {
        let handle = ShmHandle::new(self.handle_counter.fetch_add(1, Ordering::Relaxed));

        let mapping = ShmMapping {
            handle,
            region_id: region.id,
            vaddr,
            size: region.size_bytes(),
            process,
        };

        region.inc_ref();
        self.mappings.write().insert(handle, mapping);

        Ok(handle)
    }

    pub fn unmap(&self, handle: ShmHandle) -> Result<(), ShmError> {
        if let Some(mapping) = self.mappings.write().remove(&handle) {
            if let Some(region) = self.regions.read().get(&mapping.region_id) {
                let refs = region.dec_ref();
                if refs == 0 {
                    drop(region);
                    self.regions.write().remove(&mapping.region_id);
                }
            }
        }
        Ok(())
    }

    pub fn share_to_process(
        &self,
        region_id: ShmId,
        target_process: ProcessId,
        rights: MemoryRights,
    ) -> Result<Arc<SharedMemoryRegion>, ShmError> {
        let region = self
            .regions
            .read()
            .get(&region_id)
            .cloned()
            .ok_or(ShmError::RegionNotFound)?;

        if !region.rights.contains(rights) {
            return Err(ShmError::PermissionDenied);
        }

        let derived = region.derive(rights, region.share_mode);
        Ok(Arc::new(derived))
    }

    pub fn destroy_region(&self, id: ShmId) -> Result<(), ShmError> {
        if self.regions.write().remove(&id).is_some() {
            Ok(())
        } else {
            Err(ShmError::RegionNotFound)
        }
    }

    pub fn region_count(&self) -> usize {
        self.regions.read().len()
    }

    pub fn mapping_count(&self) -> usize {
        self.mappings.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shm_region_creation() {
        let owner = ProcessId::new();
        let region = SharedMemoryRegion::with_size(4, owner);

        assert_eq!(region.page_count, 4);
        assert_eq!(region.size_bytes(), 16384);
    }

    #[test]
    fn test_shm_region_derive() {
        let owner = ProcessId::new();
        let region = SharedMemoryRegion::with_size(2, owner);

        let derived = region.derive(MemoryRights::READ, ShareMode::ReadOnly);

        assert!(derived.rights.contains(MemoryRights::READ));
        assert!(!derived.rights.contains(MemoryRights::WRITE));
    }

    #[test]
    fn test_shm_manager() {
        let manager = SharedMemoryManager::new();
        let owner = ProcessId::new();

        let region = manager
            .create_region(
                4,
                MemoryRights::READ | MemoryRights::WRITE,
                ShareMode::ReadWrite,
                owner,
            )
            .unwrap();

        assert_eq!(manager.region_count(), 1);

        let handle = manager
            .map_region(&region, owner, VirtAddr::new(0x10000000))
            .unwrap();
        assert_eq!(manager.mapping_count(), 1);

        manager.unmap(handle).unwrap();
        assert_eq!(manager.mapping_count(), 0);
    }
}
