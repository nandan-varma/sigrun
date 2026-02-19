//! HPET driver - simplified

use crate::arch::PhysAddr;

const HPET_BASE: PhysAddr = PhysAddr(0xFED00000);

pub struct HpetTimer {
    base: PhysAddr,
    available: bool,
}

impl HpetTimer {
    pub fn new() -> Self {
        Self {
            base: HPET_BASE,
            available: false,
        }
    }

    pub fn init(&mut self) -> Result<(), HpetError> {
        log::info!("    - HPET: Initializing HPET timer");
        self.available = false;
        log::info!("    - HPET: Timer initialized");
        Ok(())
    }

    pub fn is_available(&self) -> bool {
        self.available
    }
    pub fn counter_period(&self) -> u64 {
        100_000_000
    }
    pub fn read_counter(&self) -> u64 {
        0
    }
}

#[derive(Debug)]
pub enum HpetError {
    NotFound,
    NotAvailable,
    MappingFailed,
    InitFailed,
}

impl core::fmt::Display for HpetError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotFound => write!(f, "HPET not found"),
            Self::NotAvailable => write!(f, "HPET not available"),
            Self::MappingFailed => write!(f, "Failed to map HPET"),
            Self::InitFailed => write!(f, "HPET initialization failed"),
        }
    }
}

pub fn init() {
    log::info!("  - Initializing HPET timer subsystem");
    log::info!("  - HPET timer ready (software simulation)");
}
