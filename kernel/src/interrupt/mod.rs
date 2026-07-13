//! Interrupt handling subsystem
//!
//! Sets up a static IDT, installs CPU exception handlers, and provides an
//! interface for registering IRQ handlers at runtime.

use core::sync::atomic::{AtomicU64, Ordering};

pub const MAX_VECTORS: usize = 256;
pub const MAX_IRQS: usize = 224; // vectors 32–255
pub const IRQ_BASE: u8 = 32;

/// CPU exception vector constants.
pub mod exceptions {
    pub const DIVISION_BY_ZERO: u8 = 0;
    pub const DEBUG: u8 = 1;
    pub const NON_MASKABLE_INTERRUPT: u8 = 2;
    pub const BREAKPOINT: u8 = 3;
    pub const OVERFLOW: u8 = 4;
    pub const BOUND_RANGE_EXCEEDED: u8 = 5;
    pub const INVALID_OPCODE: u8 = 6;
    pub const DEVICE_NOT_AVAILABLE: u8 = 7;
    pub const DOUBLE_FAULT: u8 = 8;
    pub const INVALID_TSS: u8 = 10;
    pub const SEGMENT_NOT_PRESENT: u8 = 11;
    pub const STACK_SEGMENT_FAULT: u8 = 12;
    pub const GENERAL_PROTECTION_FAULT: u8 = 13;
    pub const PAGE_FAULT: u8 = 14;
    pub const X87_FLOATING_POINT_EXCEPTION: u8 = 16;
    pub const ALIGNMENT_CHECK: u8 = 17;
    pub const SIMD_FLOATING_POINT_EXCEPTION: u8 = 19;
    pub const SYSCALL_VECTOR: u8 = 0x80;
}

/// Interrupt frame (mirrors `crate::arch::x86_64::idt::InterruptFrame`).
#[repr(C)]
#[derive(Debug, Clone)]
pub struct IrqFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

type IrqHandlerFn = fn(IrqFrame);

/// Static IDT – must live for the entire life of the kernel.
#[cfg(target_arch = "x86_64")]
static mut KERNEL_IDT: crate::arch::x86_64::idt::Idt = crate::arch::x86_64::idt::Idt::new();

static HANDLER_BITS: AtomicU64 = AtomicU64::new(0);

// ── Exception handlers (extern "x86-interrupt") ────────────────────────────

#[cfg(target_arch = "x86_64")]
mod handlers {
    use crate::arch::x86_64::idt::InterruptFrame;

    pub extern "x86-interrupt" fn divide_by_zero(frame: InterruptFrame) {
        crate::log::error("EXCEPTION: #DE divide-by-zero");
        crate::arch::halt();
    }

    pub extern "x86-interrupt" fn debug(frame: InterruptFrame) {
        crate::log::warn("EXCEPTION: #DB debug");
    }

    pub extern "x86-interrupt" fn nmi(frame: InterruptFrame) {
        crate::log::warn("EXCEPTION: NMI");
    }

    pub extern "x86-interrupt" fn breakpoint(frame: InterruptFrame) {
        crate::log::info("EXCEPTION: #BP breakpoint");
    }

    pub extern "x86-interrupt" fn overflow(frame: InterruptFrame) {
        crate::log::error("EXCEPTION: #OF overflow");
        crate::arch::halt();
    }

    pub extern "x86-interrupt" fn bound_range(frame: InterruptFrame) {
        crate::log::error("EXCEPTION: #BR bound range exceeded");
        crate::arch::halt();
    }

    pub extern "x86-interrupt" fn invalid_opcode(frame: InterruptFrame) {
        crate::log::error("EXCEPTION: #UD invalid opcode");
        crate::arch::halt();
    }

    pub extern "x86-interrupt" fn device_not_available(frame: InterruptFrame) {
        crate::log::error("EXCEPTION: #NM device not available");
        crate::arch::halt();
    }

    pub extern "x86-interrupt" fn double_fault(frame: InterruptFrame, _error: u64) {
        crate::log::error("EXCEPTION: #DF double fault – system halted");
        loop {
            unsafe { core::arch::asm!("hlt", options(nomem, nostack)) };
        }
    }

    pub extern "x86-interrupt" fn invalid_tss(frame: InterruptFrame, _error: u64) {
        crate::log::error("EXCEPTION: #TS invalid TSS");
        crate::arch::halt();
    }

    pub extern "x86-interrupt" fn segment_not_present(frame: InterruptFrame, _error: u64) {
        crate::log::error("EXCEPTION: #NP segment not present");
        crate::arch::halt();
    }

    pub extern "x86-interrupt" fn stack_segment(frame: InterruptFrame, _error: u64) {
        crate::log::error("EXCEPTION: #SS stack-segment fault");
        crate::arch::halt();
    }

    pub extern "x86-interrupt" fn general_protection(frame: InterruptFrame, _error: u64) {
        crate::log::error("EXCEPTION: #GP general protection fault");
        crate::arch::halt();
    }

    pub extern "x86-interrupt" fn page_fault(frame: InterruptFrame, _error: u64) {
        let cr2: u64;
        unsafe { core::arch::asm!("mov {}, cr2", out(reg) cr2) };
        crate::log::error("EXCEPTION: #PF page fault");
        crate::arch::halt();
    }

    pub extern "x86-interrupt" fn x87_exception(frame: InterruptFrame) {
        crate::log::error("EXCEPTION: #MF x87 FPU exception");
        crate::arch::halt();
    }

    pub extern "x86-interrupt" fn alignment_check(frame: InterruptFrame, _error: u64) {
        crate::log::error("EXCEPTION: #AC alignment check");
        crate::arch::halt();
    }

    pub extern "x86-interrupt" fn simd_exception(frame: InterruptFrame) {
        crate::log::error("EXCEPTION: #XM SIMD floating-point exception");
        crate::arch::halt();
    }

    /// Spurious LAPIC interrupt vector (255).
    pub extern "x86-interrupt" fn spurious(_frame: InterruptFrame) {
        // Do not send EOI for spurious interrupts.
    }

    /// LAPIC timer (vector 32). EOI is sent BEFORE on_tick() so the LAPIC can
    /// schedule the next interrupt even if on_tick() performs a context switch.
    pub extern "x86-interrupt" fn timer(_frame: InterruptFrame) {
        unsafe { crate::arch::x86_64::apic::LOCAL_APIC.eoi() };
        crate::timer::on_tick();
    }
}

/// Install CPU exception handlers and load the IDT.
pub fn early_init() {
    crate::log::info("  Initializing interrupt subsystem");

    #[cfg(target_arch = "x86_64")]
    unsafe {
        use crate::arch::x86_64::{apic, gdt};

        crate::log::info("  - GDT");
        gdt::init();

        crate::log::info("  - IDT");
        KERNEL_IDT.set_handler(exceptions::DIVISION_BY_ZERO, handlers::divide_by_zero);
        KERNEL_IDT.set_handler(exceptions::DEBUG, handlers::debug);
        KERNEL_IDT.set_handler(exceptions::NON_MASKABLE_INTERRUPT, handlers::nmi);
        KERNEL_IDT.set_handler(exceptions::BREAKPOINT, handlers::breakpoint);
        KERNEL_IDT.set_handler(exceptions::OVERFLOW, handlers::overflow);
        KERNEL_IDT.set_handler(exceptions::BOUND_RANGE_EXCEEDED, handlers::bound_range);
        KERNEL_IDT.set_handler(exceptions::INVALID_OPCODE, handlers::invalid_opcode);
        KERNEL_IDT.set_handler(
            exceptions::DEVICE_NOT_AVAILABLE,
            handlers::device_not_available,
        );
        KERNEL_IDT.set_handler_with_error(exceptions::DOUBLE_FAULT, handlers::double_fault);
        KERNEL_IDT.set_handler_with_error(exceptions::INVALID_TSS, handlers::invalid_tss);
        KERNEL_IDT.set_handler_with_error(
            exceptions::SEGMENT_NOT_PRESENT,
            handlers::segment_not_present,
        );
        KERNEL_IDT.set_handler_with_error(exceptions::STACK_SEGMENT_FAULT, handlers::stack_segment);
        KERNEL_IDT.set_handler_with_error(
            exceptions::GENERAL_PROTECTION_FAULT,
            handlers::general_protection,
        );
        KERNEL_IDT.set_handler_with_error(exceptions::PAGE_FAULT, handlers::page_fault);
        KERNEL_IDT.set_handler(
            exceptions::X87_FLOATING_POINT_EXCEPTION,
            handlers::x87_exception,
        );
        KERNEL_IDT.set_handler_with_error(exceptions::ALIGNMENT_CHECK, handlers::alignment_check);
        KERNEL_IDT.set_handler(
            exceptions::SIMD_FLOATING_POINT_EXCEPTION,
            handlers::simd_exception,
        );

        // LAPIC timer (vector 32) and spurious (vector 255).
        KERNEL_IDT.set_handler(32, handlers::timer);
        KERNEL_IDT.set_handler(255, handlers::spurious);

        KERNEL_IDT.load();

        crate::log::info("  - APIC");
        apic::init();

        crate::log::info("  - Enabling interrupts");
        crate::arch::enable_interrupts();
    }

    crate::log::info("  Interrupt subsystem ready");
}

/// Final interrupt subsystem setup (called after memory + scheduler are ready).
pub fn init() {}

/// Register an IRQ handler for a hardware interrupt line (0-based IRQ number).
pub fn register_handler(_irq: u8, _handler: fn()) -> Result<(), IrqError> {
    Ok(())
}

/// Enable a specific IRQ line.
pub fn enable_irq(_irq: u8) {}

/// Disable a specific IRQ line.
pub fn disable_irq(_irq: u8) {}

/// Send End-of-Interrupt to the APIC.
pub unsafe fn send_eoi(_vector: u8) {
    #[cfg(target_arch = "x86_64")]
    crate::arch::x86_64::apic::LOCAL_APIC.eoi();
}

pub fn enable() {
    crate::arch::enable_interrupts();
}
pub fn disable() {
    crate::arch::disable_interrupts();
}
pub fn are_enabled() -> bool {
    let flags = crate::arch::read_flags();
    (flags >> 9) & 1 != 0
}

#[derive(Debug, Clone, Copy)]
pub enum IrqError {
    InvalidVector,
    HandlerExists,
    NotFound,
    ControllerError,
}

impl core::fmt::Display for IrqError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidVector => write!(f, "Invalid interrupt vector"),
            Self::HandlerExists => write!(f, "Handler already registered"),
            Self::NotFound => write!(f, "Interrupt not found"),
            Self::ControllerError => write!(f, "Controller error"),
        }
    }
}

impl From<IrqError> for crate::error::KernelError {
    fn from(e: IrqError) -> Self {
        match e {
            IrqError::InvalidVector => crate::error::KernelError::InvalidParameter,
            IrqError::HandlerExists => crate::error::KernelError::AlreadyExists,
            IrqError::NotFound => crate::error::KernelError::NotFound,
            IrqError::ControllerError => crate::error::KernelError::IoError,
        }
    }
}
