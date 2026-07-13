//! Global Descriptor Table (GDT) for x86_64
//!
//! Provides segmentation setup for long mode with TSS support.

use core::mem::size_of;
use core::sync::atomic::AtomicBool;

/// Number of GDT entries
pub const GDT_ENTRIES: usize = 7;

/// GDT structure
#[repr(C, align(16))]
pub struct Gdt {
    entries: [GdtEntry; GDT_ENTRIES],
}

impl Gdt {
    /// Create a new GDT with standard entries.
    ///
    /// The user_data/user_code order (0x18/0x20) matches the STAR MSR layout
    /// required for SYSRETQ: SS = STAR[63:48]+8 = 0x18 (user_data),
    /// CS = STAR[63:48]+16 = 0x20 (user_code).
    pub const fn new() -> Self {
        Self {
            entries: [
                GdtEntry::null(),        // 0x00: Null
                GdtEntry::kernel_code(), // 0x08: Kernel code  (DPL=0)
                GdtEntry::kernel_data(), // 0x10: Kernel data  (DPL=0)
                GdtEntry::user_data(),   // 0x18: User data    (DPL=3) ← for SYSRET SS
                GdtEntry::user_code(),   // 0x20: User code    (DPL=3) ← for SYSRET CS
                GdtEntry::null(),        // 0x28: TSS (low)
                GdtEntry::null(),        // 0x30: TSS (high)
            ],
        }
    }

    /// Set TSS descriptor in GDT
    pub fn set_tss(&mut self, tss: &Tss) {
        let tss_addr = tss as *const _ as u64;
        let limit = size_of::<Tss>() - 1;

        let low = ((limit & 0xFFFF) as u64)
            | ((tss_addr & 0xFFFF) << 16)
            | (((tss_addr >> 16) & 0xFF) << 32)
            | (0x89u64 << 40)
            | (((limit >> 16) as u64 & 0xF) << 48)
            | ((tss_addr >> 24) & 0xFF) << 56;

        let high = tss_addr >> 32;

        self.entries[5] = GdtEntry(low);
        self.entries[6] = GdtEntry(high);
    }

    /// Load the GDT and update segment registers
    pub unsafe fn load(&'static self) {
        let gdtr = Gdtr {
            limit: (size_of::<Self>() - 1) as u16,
            base: self as *const _ as u64,
        };

        core::arch::asm!(
            "lgdt [{0}]",
            in(reg) &gdtr,
            options(readonly, nostack, preserves_flags)
        );
    }
}

/// Single GDT entry (64-bit)
#[derive(Clone, Copy)]
#[repr(C)]
pub struct GdtEntry(u64);

impl GdtEntry {
    /// Create a null entry
    pub const fn null() -> Self {
        Self(0)
    }

    /// Create a kernel code segment entry
    pub const fn kernel_code() -> Self {
        Self(0x00AF9A000000FFFF)
    }

    /// Create a kernel data segment entry
    pub const fn kernel_data() -> Self {
        Self(0x00CF92000000FFFF)
    }

    /// Create a user code segment entry
    pub const fn user_code() -> Self {
        Self(0x00AFFA000000FFFF)
    }

    /// Create a user data segment entry
    pub const fn user_data() -> Self {
        Self(0x00CFF2000000FFFF)
    }
}

/// Task State Segment
#[repr(C, packed)]
pub struct Tss {
    reserved0: u32,
    /// Ring 0-2 stack pointers
    pub rsp: [u64; 3],
    reserved1: u64,
    /// Interrupt stack table pointers
    pub ist: [u64; 7],
    reserved2: u64,
    reserved3: u16,
    /// I/O map base offset
    iomap_base: u16,
}

impl Tss {
    /// Create a new empty TSS
    pub const fn new() -> Self {
        Self {
            reserved0: 0,
            rsp: [0; 3],
            reserved1: 0,
            ist: [0; 7],
            reserved2: 0,
            reserved3: 0,
            iomap_base: size_of::<Tss>() as u16,
        }
    }

    /// Set the stack pointer for a privilege level
    pub fn set_rsp(&mut self, privilege: usize, stack: u64) {
        if privilege < 3 {
            self.rsp[privilege] = stack;
        }
    }

    /// Set an interrupt stack table entry
    pub fn set_ist(&mut self, index: usize, stack: u64) {
        if index > 0 && index <= 7 {
            self.ist[index - 1] = stack;
        }
    }
}

/// GDT register structure
#[repr(C, packed)]
pub struct Gdtr {
    pub limit: u16,
    pub base: u64,
}

impl Gdtr {
    /// Create a GDTR for a GDT
    pub fn new(gdt: &Gdt) -> Self {
        Self {
            limit: (size_of::<Gdt>() - 1) as u16,
            base: gdt as *const _ as u64,
        }
    }
}

/// Segment selectors (GDT byte offsets, with RPL in bits [1:0]).
pub struct SegmentSelector(pub u16);

impl SegmentSelector {
    pub const KERNEL_CODE: Self = Self(0x08);
    pub const KERNEL_DATA: Self = Self(0x10);
    /// 0x18 | 3 = 0x1b – user data (DPL=3); also SYSRET SS target.
    pub const USER_DATA: Self = Self(0x18 | 3);
    /// 0x20 | 3 = 0x23 – user code (DPL=3); also SYSRET CS target.
    pub const USER_CODE: Self = Self(0x20 | 3);
    pub const TSS: Self = Self(0x28);
}

/// Load GDT from pointer (unsafe)
pub unsafe fn load_gdt(gdtr: &Gdtr) {
    core::arch::asm!(
        "lgdt [{}]",
        in(reg) gdtr,
        options(readonly, nostack, preserves_flags)
    );
}

/// Load code segment register
pub unsafe fn load_cs(selector: SegmentSelector) {
    core::arch::asm!(
        "push {sel}",
        "lea {tmp}, [2f + rip]",
        "push {tmp}",
        "retfq",
        "2:",
        sel = in(reg) selector.0 as u64,
        tmp = lateout(reg) _,
        options(preserves_flags)
    );
}

/// Load stack segment register
pub unsafe fn load_ss(selector: SegmentSelector) {
    core::arch::asm!(
        "mov ss, ax",
        in("ax") selector.0,
        options(nomem, nostack, preserves_flags)
    );
}

/// Load data segment registers (DS, ES, FS, GS)
pub unsafe fn load_data_segments(selector: SegmentSelector) {
    core::arch::asm!(
        "mov ds, ax",
        "mov es, ax",
        in("ax") selector.0,
        options(nomem, nostack, preserves_flags)
    );
}

/// Load task register
pub unsafe fn load_tss(selector: SegmentSelector) {
    core::arch::asm!(
        "ltr ax",
        in("ax") selector.0,
        options(nomem, nostack, preserves_flags)
    );
}

// ── Static interrupt stack for ring-3 → ring-0 transitions ───────────────────
const IST_STACK_SIZE: usize = 8192;
#[repr(align(16))]
struct IstStack([u8; IST_STACK_SIZE]);
static mut IST_STACK_BUF: IstStack = IstStack([0; IST_STACK_SIZE]);

// Static GDT and TSS
static mut GDT: Gdt = Gdt::new();
static mut TSS: Tss = Tss::new();
static GDT_INITIALIZED: AtomicBool = AtomicBool::new(false);

/// Set TSS.RSP0 to `stack_top` (called after allocating a real kernel stack).
pub fn set_tss_rsp0(stack_top: u64) {
    unsafe { TSS.set_rsp(0, stack_top) }
}

/// Initialize the GDT with kernel/user segments, load it, and configure TSS.
pub fn init() {
    if GDT_INITIALIZED.load(core::sync::atomic::Ordering::Relaxed) {
        return;
    }

    unsafe {
        // Interrupt stack for ring-3→ring-0 CPU-pushed interrupt frames.
        let ist_top = IST_STACK_BUF.0.as_ptr().add(IST_STACK_SIZE) as u64;
        TSS.set_rsp(0, ist_top); // used by hardware on ring-3 interrupt
        TSS.set_ist(1, ist_top); // IST1 – double fault

        GDT.set_tss(&TSS);
        GDT.load();

        load_cs(SegmentSelector::KERNEL_CODE);
        load_ss(SegmentSelector::KERNEL_DATA);
        load_data_segments(SegmentSelector::KERNEL_DATA);
        load_tss(SegmentSelector::TSS);
    }

    GDT_INITIALIZED.store(true, core::sync::atomic::Ordering::Relaxed);
}

/// Return the top of the IST stack (for informational logging).
pub fn ist_stack_top() -> u64 {
    unsafe { IST_STACK_BUF.0.as_ptr().add(IST_STACK_SIZE) as u64 }
}
