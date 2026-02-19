//! Scheduler subsystem
//! 
//! Provides task scheduling, priority management, and CPU allocation.

use crate::error::KernelError;

/// Initialize the scheduler
pub fn init() -> Scheduler {
    Scheduler::new()
}

/// Main scheduler structure
pub struct Scheduler {
    cpu_count: usize,
}

impl Scheduler {
    pub fn new() -> Self {
        Self {
            cpu_count: 1, // Would detect actual CPU count
        }
    }
    
    /// Get number of CPUs
    pub fn cpu_count(&self) -> usize {
        self.cpu_count
    }
}

/// Task ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
    pub fn new() -> Self {
        use core::sync::atomic::{AtomicU64, Ordering};
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
    
    pub fn as_u64(self) -> u64 { self.0 }
}

/// Task priority (0 = highest, 255 = lowest)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(u8);

impl Priority {
    pub const MAX: Priority = Priority(0);
    pub const MIN: Priority = Priority(255);
    pub const DEFAULT: Priority = Priority(128);
    
    pub fn new(p: u8) -> Self { Self(p) }
    pub fn as_u8(self) -> u8 { self.0 }
}

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked,
    Sleeping,
    Terminated,
}

/// Create initial userspace process
pub fn create_init_process() -> TaskId {
    // Simplified: Would create actual process with capabilities
    log::info!("    Creating PID 1 (init)");
    TaskId::new()
}

/// Start the scheduler (enters idle loop)
pub fn start() -> ! {
    log::info!("    Scheduler running on CPU 0");
    loop {
        // Idle loop - in real implementation, halt CPU
        crate::arch::halt();
    }
}
