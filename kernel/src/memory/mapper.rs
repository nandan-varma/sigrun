//! Mapper trait for address space operations
//!
//! Provides the core interface for mapping, unmapping, and modifying page tables

use super::{
    flush_tlb, page_table_indices, MemoryError, PageTable, PageTableFlags, PhysFrame, Pml4,
};
use crate::arch::VirtAddr;

#[derive(Debug, Clone, Copy)]
pub struct PageQuery {
    pub present: bool,
    pub frame: Option<PhysFrame>,
    pub flags: PageTableFlags,
}

pub trait Mapper {
    fn map(
        &mut self,
        virt: VirtAddr,
        frame: PhysFrame,
        flags: PageTableFlags,
    ) -> Result<(), MemoryError>;
    fn unmap(&mut self, virt: VirtAddr) -> Result<PhysFrame, MemoryError>;
    fn update_flags(&mut self, virt: VirtAddr, flags: PageTableFlags) -> Result<(), MemoryError>;
    fn query(&self, virt: VirtAddr) -> Result<PageQuery, MemoryError>;
    fn translate(&self, virt: VirtAddr) -> Option<PhysFrame>;
}

pub struct PageTableMapper {
    pml4: &'static mut Pml4,
    allocator: fn() -> Option<PhysFrame>,
}

impl PageTableMapper {
    pub unsafe fn new(pml4_phys: PhysFrame, allocator: fn() -> Option<PhysFrame>) -> Self {
        let pml4 = &mut *(pml4_phys.start.as_mut_ptr::<Pml4>());
        Self { pml4, allocator }
    }

    unsafe fn allocate_table(&mut self) -> Result<PhysFrame, MemoryError> {
        (self.allocator)().ok_or(MemoryError::FrameAllocationFailed)
    }

    unsafe fn get_or_create_pdpt(
        &mut self,
        pml4_idx: usize,
    ) -> Result<&'static mut PageTable, MemoryError> {
        if !self.pml4.entries[pml4_idx].is_present() {
            let frame = self.allocate_table()?;
            let new_table = &mut *(frame.start.as_mut_ptr::<PageTable>());
            new_table.clear();
            self.pml4.entries[pml4_idx].set(frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
        }

        Ok(&mut *(self.pml4.entries[pml4_idx].frame().start.as_mut_ptr::<PageTable>()))
    }

    unsafe fn get_or_create_pd(
        &mut self,
        pdpt: &'static mut PageTable,
        pdpt_idx: usize,
    ) -> Result<&'static mut PageTable, MemoryError> {
        if !pdpt.entries[pdpt_idx].is_present() {
            let frame = self.allocate_table()?;
            let new_table = &mut *(frame.start.as_mut_ptr::<PageTable>());
            new_table.clear();
            pdpt.entries[pdpt_idx].set(frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
        }

        if pdpt.entries[pdpt_idx].is_huge() {
            return Err(MemoryError::AlreadyMapped);
        }

        Ok(&mut *(pdpt.entries[pdpt_idx].frame().start.as_mut_ptr::<PageTable>()))
    }

    unsafe fn get_or_create_pt(
        &mut self,
        pd: &'static mut PageTable,
        pd_idx: usize,
    ) -> Result<&'static mut PageTable, MemoryError> {
        if !pd.entries[pd_idx].is_present() {
            let frame = self.allocate_table()?;
            let new_table = &mut *(frame.start.as_mut_ptr::<PageTable>());
            new_table.clear();
            pd.entries[pd_idx].set(frame, PageTableFlags::PRESENT | PageTableFlags::WRITABLE);
        }

        if pd.entries[pd_idx].is_huge() {
            return Err(MemoryError::AlreadyMapped);
        }

        Ok(&mut *(pd.entries[pd_idx].frame().start.as_mut_ptr::<PageTable>()))
    }
}

impl Mapper for PageTableMapper {
    fn map(
        &mut self,
        virt: VirtAddr,
        frame: PhysFrame,
        flags: PageTableFlags,
    ) -> Result<(), MemoryError> {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = page_table_indices(virt);

        unsafe {
            let pdpt = self.get_or_create_pdpt(pml4_idx)?;
            let pd = self.get_or_create_pd(pdpt, pdpt_idx)?;
            let pt = self.get_or_create_pt(pd, pd_idx)?;

            let entry = &mut pt.entries[pt_idx];
            if entry.is_present() {
                return Err(MemoryError::AlreadyMapped);
            }

            entry.set(frame, flags | PageTableFlags::PRESENT);
            flush_tlb(virt);
        }

        Ok(())
    }

    fn unmap(&mut self, virt: VirtAddr) -> Result<PhysFrame, MemoryError> {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = page_table_indices(virt);

        unsafe {
            let pml4_entry = &self.pml4.entries[pml4_idx];
            if !pml4_entry.is_present() {
                return Err(MemoryError::NotMapped);
            }

            let pdpt = &mut *(pml4_entry.frame().start.as_mut_ptr::<PageTable>());
            let pdpt_entry = &pdpt.entries[pdpt_idx];
            if !pdpt_entry.is_present() {
                return Err(MemoryError::NotMapped);
            }

            let pd = &mut *(pdpt_entry.frame().start.as_mut_ptr::<PageTable>());
            let pd_entry = &pd.entries[pd_idx];
            if !pd_entry.is_present() {
                return Err(MemoryError::NotMapped);
            }

            if pd_entry.is_huge() {
                return Err(MemoryError::AlreadyMapped);
            }

            let pt = &mut *(pd_entry.frame().start.as_mut_ptr::<PageTable>());
            let pt_entry = &mut pt.entries[pt_idx];
            if !pt_entry.is_present() {
                return Err(MemoryError::NotMapped);
            }

            let frame = pt_entry.frame();
            pt_entry.clear();
            flush_tlb(virt);

            Ok(frame)
        }
    }

    fn update_flags(&mut self, virt: VirtAddr, flags: PageTableFlags) -> Result<(), MemoryError> {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = page_table_indices(virt);

        unsafe {
            let pml4_entry = &self.pml4.entries[pml4_idx];
            if !pml4_entry.is_present() {
                return Err(MemoryError::NotMapped);
            }

            let pdpt = &*(pml4_entry.frame().start.as_ptr::<PageTable>());
            let pdpt_entry = &pdpt.entries[pdpt_idx];
            if !pdpt_entry.is_present() {
                return Err(MemoryError::NotMapped);
            }

            let pd = &*(pdpt_entry.frame().start.as_ptr::<PageTable>());
            let pd_entry = &pd.entries[pd_idx];
            if !pd_entry.is_present() {
                return Err(MemoryError::NotMapped);
            }

            if pd_entry.is_huge() {
                return Err(MemoryError::AlreadyMapped);
            }

            let pt = &mut *(pd_entry.frame().start.as_mut_ptr::<PageTable>());
            let pt_entry = &mut pt.entries[pt_idx];
            if !pt_entry.is_present() {
                return Err(MemoryError::NotMapped);
            }

            let frame = pt_entry.frame();
            pt_entry.set(frame, flags | PageTableFlags::PRESENT);
            flush_tlb(virt);
        }

        Ok(())
    }

    fn query(&self, virt: VirtAddr) -> Result<PageQuery, MemoryError> {
        let (pml4_idx, pdpt_idx, pd_idx, pt_idx) = page_table_indices(virt);

        unsafe {
            let pml4_entry = &self.pml4.entries[pml4_idx];
            if !pml4_entry.is_present() {
                return Ok(PageQuery {
                    present: false,
                    frame: None,
                    flags: PageTableFlags::empty(),
                });
            }

            let pdpt = &*(pml4_entry.frame().start.as_ptr::<PageTable>());
            let pdpt_entry = &pdpt.entries[pdpt_idx];
            if !pdpt_entry.is_present() {
                return Ok(PageQuery {
                    present: false,
                    frame: None,
                    flags: PageTableFlags::empty(),
                });
            }

            let pd = &*(pdpt_entry.frame().start.as_ptr::<PageTable>());
            let pd_entry = &pd.entries[pd_idx];
            if !pd_entry.is_present() {
                return Ok(PageQuery {
                    present: false,
                    frame: None,
                    flags: PageTableFlags::empty(),
                });
            }

            if pd_entry.is_huge() {
                return Ok(PageQuery {
                    present: true,
                    frame: Some(pd_entry.frame()),
                    flags: pd_entry.flags(),
                });
            }

            let pt = &*(pd_entry.frame().start.as_ptr::<PageTable>());
            let pt_entry = &pt.entries[pt_idx];

            Ok(PageQuery {
                present: pt_entry.is_present(),
                frame: if pt_entry.is_present() {
                    Some(pt_entry.frame())
                } else {
                    None
                },
                flags: pt_entry.flags(),
            })
        }
    }

    fn translate(&self, virt: VirtAddr) -> Option<PhysFrame> {
        match self.query(virt) {
            Ok(query) if query.present => query.frame,
            _ => None,
        }
    }
}
