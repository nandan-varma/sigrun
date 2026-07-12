//! Interrupt Descriptor Table for x86_64

use core::mem::size_of;

#[repr(C, align(16))]
pub struct Idt {
    entries: [IdtEntry; 256],
}

impl Idt {
    pub const fn new() -> Self {
        Self {
            entries: [IdtEntry::empty(); 256],
        }
    }

    /// Register a handler that receives only the interrupt frame (no error code).
    pub fn set_handler(
        &mut self,
        vector: u8,
        handler: extern "x86-interrupt" fn(InterruptFrame),
    ) {
        let addr = handler as u64;
        self.entries[vector as usize] = IdtEntry::new(addr, 0x08, 0x8E);
    }

    /// Register a handler for exceptions that push an error code.
    pub fn set_handler_with_error(
        &mut self,
        vector: u8,
        handler: extern "x86-interrupt" fn(InterruptFrame, u64),
    ) {
        let addr = handler as u64;
        self.entries[vector as usize] = IdtEntry::new(addr, 0x08, 0x8E);
    }

    /// Register a handler using a raw function address (for assembly stubs).
    pub unsafe fn set_handler_raw(&mut self, vector: u8, addr: u64) {
        self.entries[vector as usize] = IdtEntry::new(addr, 0x08, 0x8E);
    }

    /// Load this IDT into the CPU.  Requires a `'static` reference so the
    /// IDT remains valid for the lifetime of the kernel.
    pub fn load(&'static self) {
        let idtr = Idtr::new(self);
        unsafe {
            core::arch::asm!(
                "lidt [{0}]",
                in(reg) &idtr,
                options(readonly, nostack, preserves_flags)
            );
        }
    }
}

#[derive(Clone, Copy)]
#[repr(C)]
pub struct IdtEntry {
    offset_low: u16,
    selector: u16,
    ist: u8,
    type_attr: u8,
    offset_mid: u16,
    offset_high: u32,
    reserved: u32,
}

impl IdtEntry {
    pub const fn empty() -> Self {
        Self {
            offset_low: 0,
            selector: 0,
            ist: 0,
            type_attr: 0,
            offset_mid: 0,
            offset_high: 0,
            reserved: 0,
        }
    }

    pub const fn new(offset: u64, selector: u16, type_attr: u8) -> Self {
        Self {
            offset_low: offset as u16,
            selector,
            ist: 0,
            type_attr,
            offset_mid: (offset >> 16) as u16,
            offset_high: (offset >> 32) as u32,
            reserved: 0,
        }
    }
}

/// The interrupt/exception frame the CPU pushes automatically.
#[repr(C)]
pub struct InterruptFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

#[repr(C, packed)]
pub struct Idtr {
    pub limit: u16,
    pub base: u64,
}

impl Idtr {
    pub fn new(idt: &Idt) -> Self {
        Self {
            limit: (size_of::<Idt>() - 1) as u16,
            base: idt as *const _ as u64,
        }
    }
}
