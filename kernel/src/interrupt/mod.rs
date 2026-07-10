//! Interrupt handling subsystem
//!
//! Provides IDT setup, interrupt routing, and handler registration.

use core::sync::atomic::{AtomicU64, Ordering};

/// Maximum number of interrupt vectors
pub const MAX_VECTORS: usize = 256;

/// Number of IRQs (hardware interrupts)
pub const MAX_IRQS: usize = 256;

/// Base vector for IRQs
pub const IRQ_BASE: u8 = 32;

/// CPU exception vectors
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
    pub const COPROCESSOR_SEGMENT_OVERRUN: u8 = 9;
    pub const INVALID_TSS: u8 = 10;
    pub const SEGMENT_NOT_PRESENT: u8 = 11;
    pub const STACK_SEGMENT_FAULT: u8 = 12;
    pub const GENERAL_PROTECTION_FAULT: u8 = 13;
    pub const PAGE_FAULT: u8 = 14;
    pub const X87_FLOATING_POINT_EXCEPTION: u8 = 16;
    pub const ALIGNMENT_CHECK: u8 = 17;
    pub const MACHINE_CHECK: u8 = 18;
    pub const SIMD_FLOATING_POINT_EXCEPTION: u8 = 19;
    pub const VIRTUALIZATION_EXCEPTION: u8 = 20;
    pub const SECURITY_EXCEPTION: u8 = 30;
    pub const SYSCALL_VECTOR: u8 = 0x80;
}

/// Handler function type for interrupts without error code
type IrqHandlerFn = fn(IrqFrame);

/// Handler function type for exceptions with error code
type ErrorHandlerFn = fn(IrqFrame, u64);

/// Interrupt frame passed to handlers
#[repr(C)]
#[derive(Debug, Clone)]
pub struct IrqFrame {
    pub rip: u64,
    pub cs: u64,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u64,
}

/// Interrupt controller interface
trait InterruptController {
    fn init(&self);
    fn enable_irq(&self, irq: u8);
    fn disable_irq(&self, irq: u8);
    fn send_eoi(&self, irq: u8);
    fn mask_all(&self);
}

/// Interrupt manager
struct InterruptManager {
    handlers: [Option<IrqHandlerFn>; MAX_VECTORS],
    error_handlers: [Option<ErrorHandlerFn>; MAX_VECTORS],
    irq_map: [Option<u8>; MAX_IRQS],
}

impl InterruptManager {
    const fn new() -> Self {
        Self {
            handlers: [None; MAX_VECTORS],
            error_handlers: [None; MAX_VECTORS],
            irq_map: [None; MAX_IRQS],
        }
    }

    fn register_handler(&mut self, vector: u8, handler: IrqHandlerFn) -> Result<(), IrqError> {
        if vector as usize >= MAX_VECTORS {
            return Err(IrqError::InvalidVector);
        }

        if self.handlers[vector as usize].is_some() {
            return Err(IrqError::HandlerExists);
        }

        self.handlers[vector as usize] = Some(handler);
        Ok(())
    }
}

static mut INTERRUPT_MANAGER: InterruptManager = InterruptManager::new();
static HANDLER_REGISTERS: AtomicU64 = AtomicU64::new(0);

/// Initialize interrupt subsystem
pub fn early_init() {
    crate::log::info_formatted("  - Initializing interrupt subsystem...");

    #[cfg(target_arch = "x86_64")]
    unsafe {
        use crate::arch::x86_64::{apic, gdt, idt::Idt};

        crate::log::info_formatted("  - Initializing GDT...");
        gdt::init();
        crate::log::info_formatted("  - GDT initialized");

        crate::log::info_formatted("  - Initializing IDT...");
        let mut idt = Idt::new();
        install_exception_handlers(&mut idt);
        idt.load();
        crate::log::info_formatted("  - IDT initialized");

        crate::log::info_formatted("  - Initializing APIC...");
        apic::init();
        crate::log::info_formatted("  - APIC initialized");

        crate::log::info_formatted("  - Enabling interrupts");
        crate::arch::enable_interrupts();
        crate::log::info_formatted("  - Interrupts enabled");
    }

    #[cfg(not(target_arch = "x86_64"))]
    {
        crate::log::info_formatted("  - Architecture not yet supported for interrupts");
    }
}

#[cfg(target_arch = "x86_64")]
unsafe fn install_exception_handlers(idt: &mut crate::arch::x86_64::idt::Idt) {
    use crate::arch::x86_64::idt::{Idt, InterruptFrame};

    extern "x86-interrupt" fn divide_by_zero_handler(frame: InterruptFrame) {
        crate::log::error_formatted("Divide by zero at {:#016x}");
    }

    extern "x86-interrupt" fn debug_handler(frame: InterruptFrame) {
        crate::log::debug_formatted("Debug exception at {:#016x}");
    }

    extern "x86-interrupt" fn nmi_handler(frame: InterruptFrame) {
        crate::log::warn_formatted("Non-maskable interrupt at {:#016x}");
    }

    extern "x86-interrupt" fn breakpoint_handler(frame: InterruptFrame) {
        crate::log::info_formatted("Breakpoint at {:#016x}");
    }

    extern "x86-interrupt" fn overflow_handler(frame: InterruptFrame) {
        crate::log::error_formatted("Overflow at {:#016x}");
    }

    extern "x86-interrupt" fn bound_range_handler(frame: InterruptFrame) {
        crate::log::error_formatted("Bound range exceeded at {:#016x}");
    }

    extern "x86-interrupt" fn invalid_opcode_handler(frame: InterruptFrame) {
        crate::log::error_formatted("Invalid opcode at {:#016x}");
    }

    extern "x86-interrupt" fn device_not_available_handler(frame: InterruptFrame) {
        crate::log::error_formatted("Device not available at {:#016x}");
    }

    extern "x86-interrupt" fn double_fault_handler(frame: InterruptFrame, error: u64) {
        crate::log::error_formatted(
            "DOUBLE FAULT at {:#016x}, error: {:#016x}",
            frame.rip,
            error
        );
        panic!("Double fault");
    }

    extern "x86-interrupt" fn invalid_tss_handler(frame: InterruptFrame, error: u64) {
        crate::log::error_formatted("Invalid TSS at {:#016x}, error: {:#016x}");
    }

    extern "x86-interrupt" fn segment_not_present_handler(frame: InterruptFrame, error: u64) {
        crate::log::error_formatted(
            "Segment not present at {:#016x}, error: {:#016x}",
            frame.rip,
            error
        );
    }

    extern "x86-interrupt" fn stack_segment_handler(frame: InterruptFrame, error: u64) {
        crate::log::error_formatted(
            "Stack segment fault at {:#016x}, error: {:#016x}",
            frame.rip,
            error
        );
    }

    extern "x86-interrupt" fn gpf_handler(frame: InterruptFrame, error: u64) {
        crate::log::error_formatted(
            "General protection fault at {:#016x}, error: {:#016x}",
            frame.rip,
            error
        );
    }

    extern "x86-interrupt" fn page_fault_handler(frame: InterruptFrame, error: u64) {
        let fault_addr = crate::arch::x86_64::paging::get_cr2();
        crate::log::error_formatted(
            "Page fault at {:#016x} (faulting address: {:#016x})",
            frame.rip,
            fault_addr
        );
        crate::log::error_formatted("  Error code: {:#016x}");
        crate::log::error_formatted("  Present:     {present}");
        crate::log::error_formatted("  Write:       {write}");
        crate::log::error_formatted("  User:        {user}");
        crate::log::error_formatted("  Reserved:    {reserved}");
        crate::log::error_formatted("  Instruction: {instruction}");
    }

    extern "x86-interrupt" fn x87_exception_handler(frame: InterruptFrame) {
        crate::log::error_formatted("x87 FPU exception at {:#016x}");
    }

    extern "x86-interrupt" fn alignment_check_handler(frame: InterruptFrame, error: u64) {
        crate::log::error_formatted(
            "Alignment check at {:#016x}, error: {:#016x}",
            frame.rip,
            error
        );
    }

    extern "x86-interrupt" fn simd_exception_handler(frame: InterruptFrame) {
        crate::log::error_formatted("SIMD exception at {:#016x}");
    }

    idt.set_handler(exceptions::DIVISION_BY_ZERO, divide_by_zero_handler);
    idt.set_handler(exceptions::DEBUG, debug_handler);
    idt.set_handler(exceptions::NON_MASKABLE_INTERRUPT, nmi_handler);
    idt.set_handler(exceptions::BREAKPOINT, breakpoint_handler);
    idt.set_handler(exceptions::OVERFLOW, overflow_handler);
    idt.set_handler(exceptions::BOUND_RANGE_EXCEEDED, bound_range_handler);
    idt.set_handler(exceptions::INVALID_OPCODE, invalid_opcode_handler);
    idt.set_handler(
        exceptions::DEVICE_NOT_AVAILABLE,
        device_not_available_handler,
    );
    idt.set_handler_with_error(exceptions::DOUBLE_FAULT, double_fault_handler);
    idt.set_handler_with_error(exceptions::INVALID_TSS, invalid_tss_handler);
    idt.set_handler_with_error(exceptions::SEGMENT_NOT_PRESENT, segment_not_present_handler);
    idt.set_handler_with_error(exceptions::STACK_SEGMENT_FAULT, stack_segment_handler);
    idt.set_handler_with_error(exceptions::GENERAL_PROTECTION_FAULT, gpf_handler);
    idt.set_handler_with_error(exceptions::PAGE_FAULT, page_fault_handler);
    idt.set_handler_with_error(exceptions::ALIGNMENT_CHECK, alignment_check_handler);
    idt.set_handler(
        exceptions::X87_FLOATING_POINT_EXCEPTION,
        x87_exception_handler,
    );
    idt.set_handler(
        exceptions::SIMD_FLOATING_POINT_EXCEPTION,
        simd_exception_handler,
    );
}

/// Complete initialization after memory and scheduler are ready
pub fn init() {
    crate::log::info_formatted("  - Completing interrupt subsystem initialization");

    #[cfg(target_arch = "x86_64")]
    unsafe {
        use crate::arch::x86_64::apic;
        for irq in 0..16u8 {
            let _ = apic::LOCAL_APIC.read_reg(apic::Register::Eoi);
        }
    }

    crate::log::info_formatted("  - Interrupt subsystem ready");
}

/// Register an interrupt handler
pub fn register_handler(irq: u8, handler: fn()) -> Result<(), IrqError> {
    if irq >= MAX_IRQS as u8 {
        return Err(IrqError::InvalidVector);
    }

    let mask = 1u64 << irq;
    if HANDLER_REGISTERS.fetch_or(mask, Ordering::SeqCst) & mask != 0 {
        return Err(IrqError::HandlerExists);
    }

    unsafe {
        let vector = IRQ_BASE + irq;
        INTERRUPT_MANAGER.handlers[vector as usize] = Some(transmute_handler(handler));
    }

    crate::log::debug_formatted("Registered handler for IRQ {}");
    Ok(())
}

/// Register a handler for a specific interrupt vector
pub fn register_vector_handler(vector: u8, handler: IrqHandlerFn) -> Result<(), IrqError> {
    unsafe { INTERRUPT_MANAGER.register_handler(vector, handler) }
}

/// Enable an interrupt
pub fn enable_irq(irq: u8) {
    if irq < MAX_IRQS as u8 {
        let vector = IRQ_BASE + irq;

        #[cfg(target_arch = "x86_64")]
        unsafe {
            use crate::arch::x86_64::apic;
            apic::LOCAL_APIC.write_reg(apic::Register::TaskPriority, 0);
        }

        crate::log::debug_formatted("Enabled IRQ {} (vector {})");
    }
}

/// Disable an interrupt
pub fn disable_irq(irq: u8) {
    if irq < MAX_IRQS as u8 {
        unsafe {
            #[cfg(target_arch = "x86_64")]
            {
                use crate::arch::x86_64::apic;
                let _ = apic::LOCAL_APIC.read_reg(apic::Register::Eoi);
            }
        }
        crate::log::debug_formatted("Disabled IRQ {}");
    }
}

/// Send End of Interrupt signal
pub unsafe fn send_eoi(vector: u8) {
    if vector >= IRQ_BASE {
        #[cfg(target_arch = "x86_64")]
        {
            use crate::arch::x86_64::apic;
            apic::LOCAL_APIC.eoi();
        }
    }
}

/// Enable interrupts on current CPU
pub fn enable() {
    crate::arch::enable_interrupts();
}

/// Disable interrupts on current CPU
pub fn disable() {
    crate::arch::disable_interrupts();
}

/// Check if interrupts are currently enabled
pub fn are_enabled() -> bool {
    let flags = crate::arch::read_flags();
    (flags >> 9) & 1 != 0
}

/// Default exception handler
fn default_exception_handler(vector: u8, frame: IrqFrame) {
    crate::log::error_formatted("Unhandled exception {} at {:#016x}");
    crate::log::error_formatted("  CS:  {:#016x}");
    crate::log::error_formatted("  RIP: {:#016x}");
    crate::log::error_formatted("  RSP: {:#016x}");
    crate::log::error_formatted("  RFL: {:#016x}");
}

/// Default exception handler with error code
fn default_exception_handler_with_error(vector: u8, frame: IrqFrame, error: u64) {
    crate::log::error_formatted("Unhandled exception at address");
    crate::log::error_formatted("  CS:  {:#016x}");
    crate::log::error_formatted("  RIP: {:#016x}");
    crate::log::error_formatted("  RSP: {:#016x}");
    crate::log::error_formatted("  ERR: {:#016x}");
}

/// Default IRQ handler
fn default_irq_handler(vector: u8, frame: IrqFrame) {
    let irq = vector - IRQ_BASE;
    crate::log::warn_formatted("Unhandled IRQ {} (vector {})");
}

/// Helper to convert function pointer types
unsafe fn transmute_handler(handler: fn()) -> IrqHandlerFn {
    core::mem::transmute::<fn(), IrqHandlerFn>(handler)
}

/// Interrupt errors
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
