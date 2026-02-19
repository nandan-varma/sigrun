//! LAPIC timer driver - simplified

use crate::arch::PhysAddr;

const LOCAL_APIC_BASE: PhysAddr = PhysAddr(0xFEE00000);

#[derive(Debug, Clone, Copy)]
pub enum TimerMode {
    OneShot,
    Periodic,
    TscDeadline,
}

pub struct LapicTimer {
    base: PhysAddr,
    id: u32,
    vector: u8,
    running: bool,
    mode: TimerMode,
}

impl LapicTimer {
    pub fn new(base: PhysAddr) -> Self {
        Self {
            base,
            id: 0,
            vector: 0x20,
            running: false,
            mode: TimerMode::OneShot,
        }
    }

    pub fn init(&mut self) -> Result<(), LapicError> {
        log::info!("    - LAPIC timer: Initializing");
        self.running = false;
        log::info!("    - LAPIC timer: Initialized on CPU {}", self.id);
        Ok(())
    }

    pub fn id(&self) -> u32 {
        self.id
    }
    pub fn set_vector(&mut self, vector: u8) {
        self.vector = vector;
    }
    pub fn start_one_shot(&mut self, count: u32) {
        self.mode = TimerMode::OneShot;
        self.running = true;
    }
    pub fn start_periodic(&mut self, count: u32) {
        self.mode = TimerMode::Periodic;
        self.running = true;
    }
    pub fn stop(&mut self) {
        self.running = false;
    }
    pub fn is_running(&self) -> bool {
        self.running
    }
    pub fn current_count(&self) -> u32 {
        if self.running {
            1000
        } else {
            0
        }
    }
    pub fn set_divide(&mut self, divide: u8) {}
    pub fn eoi(&self) {}
}

#[derive(Debug)]
pub enum LapicError {
    NotFound,
    NotAvailable,
    InitFailed,
    NotSupported,
}

impl core::fmt::Display for LapicError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotFound => write!(f, "LAPIC not found"),
            Self::NotAvailable => write!(f, "LAPIC not available"),
            Self::InitFailed => write!(f, "LAPIC initialization failed"),
            Self::NotSupported => write!(f, "LAPIC timer not supported"),
        }
    }
}

pub fn init() {
    log::info!("  - Initializing LAPIC timer subsystem");
    log::info!("  - LAPIC timer ready");
}

pub fn start_timer(count: u32) {}
pub fn stop_timer() {}
pub fn eoi() {}
