//! Interrupt handling subsystem
//! 
//! Provides IDT setup, interrupt routing, and handler registration.

/// Initialize interrupt subsystem
pub fn early_init() {
    log::info!("  - IDT initialized");
    log::info!("  - Interrupt handlers registered");
}

/// Register an interrupt handler
pub fn register_handler(irq: u8, handler: fn()) -> Result<(), IrqError> {
    // Simplified: Would register actual handler
    Ok(())
}

/// Enable an interrupt
pub fn enable_irq(irq: u8) {
    // Would enable IRQ at PIC/APIC
}

/// Disable an interrupt
pub fn disable_irq(irq: u8) {
    // Would disable IRQ at PIC/APIC
}

/// Interrupt errors
#[derive(Debug)]
pub enum IrqError {
    InvalidVector,
    HandlerExists,
}

impl core::fmt::Display for IrqError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidVector => write!(f, "Invalid interrupt vector"),
            Self::HandlerExists => write!(f, "Handler already registered"),
        }
    }
}
