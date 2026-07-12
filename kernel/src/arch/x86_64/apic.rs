//! Advanced Programmable Interrupt Controller (APIC) support
//!
//! Provides Local APIC and I/O APIC handling.

use core::sync::atomic::{AtomicPtr, Ordering};

pub const LOCAL_APIC_BASE: u64 = 0xFEE0_0000;
pub const IO_APIC_BASE: u64 = 0xFEC0_0000;

pub const APIC_TIMER_VECTOR: u8 = 32;
pub const APIC_SPURIOUS_VECTOR: u8 = 255;

pub struct LocalApic {
    base: AtomicPtr<u32>,
}

impl LocalApic {
    pub const fn new() -> Self {
        Self {
            base: AtomicPtr::new(LOCAL_APIC_BASE as *mut u32),
        }
    }

    pub unsafe fn init(&self, base_addr: u64) {
        self.base.store(base_addr as *mut u32, Ordering::SeqCst);

        self.write_reg(Register::Spurious, APIC_SPURIOUS_VECTOR as u32 | 0x100);

        self.write_reg(Register::TaskPriority, 0);

        self.write_reg(Register::TimerDivide, 0xB);

        self.write_reg(Register::TimerInit, 0);
        self.write_reg(Register::TimerVector, APIC_TIMER_VECTOR as u32);

        self.write_reg(Register::Lint0, 0x10000);
        self.write_reg(Register::Lint1, 0x10000);

        self.write_reg(Register::Error, 0);

        self.enable();
    }

    pub unsafe fn enable(&self) {
        let mut val = self.read_reg(Register::Spurious);
        val |= 0x100;
        self.write_reg(Register::Spurious, val);
    }

    pub unsafe fn disable(&self) {
        let mut val = self.read_reg(Register::Spurious);
        val &= !0x100;
        self.write_reg(Register::Spurious, val);
    }

    pub fn id(&self) -> u32 {
        unsafe { self.read_reg(Register::Id) >> 24 }
    }

    pub unsafe fn eoi(&self) {
        self.write_reg(Register::Eoi, 0);
    }

    pub unsafe fn send_ipi(&self, dest: u32, vector: u8) {
        let icr_low = (dest as u32) << 18 | 0x4000 | vector as u32;
        self.write_reg(Register::IcrLow, icr_low);
    }

    pub unsafe fn set_timer(&self, initial_count: u32) {
        self.write_reg(Register::TimerInit, initial_count);
        self.write_reg(Register::TimerCurrent, 0);
    }

    pub unsafe fn read_reg(&self, reg: Register) -> u32 {
        let base = self.base.load(Ordering::SeqCst);
        core::ptr::read_volatile(base.add(reg.offset()))
    }

    pub unsafe fn write_reg(&self, reg: Register, value: u32) {
        let base = self.base.load(Ordering::SeqCst);
        core::ptr::write_volatile(base.add(reg.offset()), value);
    }
}

#[repr(usize)]
pub enum Register {
    Id = 0x20,
    Version = 0x30,
    TaskPriority = 0x80,
    Eoi = 0xB0,
    Spurious = 0xF0,
    Error = 0x280,
    IcrLow = 0x300,
    IcrHigh = 0x310,
    TimerVector = 0x320,
    TimerDivide = 0x3E0,
    TimerInit = 0x380,
    TimerCurrent = 0x390,
    Lint0 = 0x350,
    Lint1 = 0x360,
}

impl Register {
    pub fn offset(self) -> usize {
        (self as usize) / 4
    }
}

pub struct IoApic {
    base: AtomicPtr<u32>,
}

impl IoApic {
    pub const fn new() -> Self {
        Self {
            base: AtomicPtr::new(IO_APIC_BASE as *mut u32),
        }
    }

    pub unsafe fn init(&self, base_addr: u64) {
        self.base.store(base_addr as *mut u32, Ordering::SeqCst);
    }

    pub unsafe fn set_irq(&self, irq: u8, vector: u8, enabled: bool) {
        let entry = (vector as u32) | if enabled { 0 } else { 0x10000 };
        let reg = 0x10 + (irq as u32) * 2;

        self.write_reg(reg, entry);
        self.write_reg(reg + 1, 0);
    }

    pub unsafe fn mask_irq(&self, irq: u8) {
        let reg = 0x10 + (irq as u32) * 2;
        let mut entry = self.read_reg(reg);
        entry |= 0x10000;
        self.write_reg(reg, entry);
    }

    pub unsafe fn unmask_irq(&self, irq: u8) {
        let reg = 0x10 + (irq as u32) * 2;
        let mut entry = self.read_reg(reg);
        entry &= !0x10000;
        self.write_reg(reg, entry);
    }

    pub unsafe fn read_reg(&self, reg: u32) -> u32 {
        let base = self.base.load(Ordering::SeqCst);
        core::ptr::write_volatile(base, reg);
        core::ptr::read_volatile(base.add(4))
    }

    pub unsafe fn write_reg(&self, reg: u32, value: u32) {
        let base = self.base.load(Ordering::SeqCst);
        core::ptr::write_volatile(base, reg);
        core::ptr::write_volatile(base.add(4), value);
    }
}

pub struct Pic {
    master_cmd: u16,
    master_data: u16,
    slave_cmd: u16,
    slave_data: u16,
}

impl Pic {
    pub const fn new() -> Self {
        Self {
            master_cmd: 0x20,
            master_data: 0x21,
            slave_cmd: 0xA0,
            slave_data: 0xA1,
        }
    }

    pub unsafe fn init(&self) {
        self.outb(self.master_cmd, 0x11);
        self.outb(self.slave_cmd, 0x11);

        self.outb(self.master_data, 0x20);
        self.outb(self.slave_data, 0x28);

        self.outb(self.master_data, 0x04);
        self.outb(self.slave_data, 0x02);

        self.outb(self.master_data, 0x01);
        self.outb(self.slave_data, 0x01);

        self.outb(self.master_data, 0xFF);
        self.outb(self.slave_data, 0xFF);
    }

    pub unsafe fn mask_all(&self) {
        self.outb(self.master_data, 0xFF);
        self.outb(self.slave_data, 0xFF);
    }

    pub unsafe fn enable_irq(&self, irq: u8) {
        let port = if irq < 8 {
            self.master_data
        } else {
            self.slave_data
        };
        let irq_bit = irq % 8;
        let mut mask = self.inb(port);
        mask &= !(1 << irq_bit);
        self.outb(port, mask);
    }

    pub unsafe fn disable_irq(&self, irq: u8) {
        let port = if irq < 8 {
            self.master_data
        } else {
            self.slave_data
        };
        let irq_bit = irq % 8;
        let mut mask = self.inb(port);
        mask |= 1 << irq_bit;
        self.outb(port, mask);
    }

    pub unsafe fn send_eoi(&self, irq: u8) {
        if irq >= 8 {
            self.outb(self.slave_cmd, 0x20);
        }
        self.outb(self.master_cmd, 0x20);
    }

    unsafe fn outb(&self, port: u16, value: u8) {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags)
        );
    }

    unsafe fn inb(&self, port: u16) -> u8 {
        let value: u8;
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags)
        );
        value
    }
}

pub static mut LOCAL_APIC: LocalApic = LocalApic::new();
pub static mut IO_APIC: IoApic = IoApic::new();
pub static mut PIC: Pic = Pic::new();

pub unsafe fn init() {
    PIC.init();

    LOCAL_APIC.init(LOCAL_APIC_BASE);

    IO_APIC.init(IO_APIC_BASE);

    PIC.mask_all();
}
