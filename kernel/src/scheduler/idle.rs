//! Idle task implementation
//!
//! Provides the system's idle loop for when no other tasks are ready.

use crate::arch::halt;
use crate::timer;

/// Idle task entry point
///
/// This function is called when the scheduler has no other work to do.
/// It enters a low-power state until the next interrupt.
pub fn idle_loop() -> ! {
    loop {
        // Check if there are any pending timer events before halting
        timer::check_timers();

        // Halt the CPU until next interrupt
        // This is an atomic check-and-halt operation
        unsafe {
            // Enable interrupts briefly to allow pending ones to fire
            crate::arch::enable_interrupts();

            // Halt the CPU - will wake on next interrupt
            halt();

            // Disable interrupts before checking state
            crate::arch::disable_interrupts();
        }

        // When we wake up, the scheduler will be invoked by the timer
        // or other interrupt handler
    }
}

/// Idle task with power management
///
/// More sophisticated idle loop that can use deeper sleep states
/// when the system has been idle for a while.
pub fn power_managed_idle() -> ! {
    const IDLE_THRESHOLD_MS: u64 = 100; // 100ms before deeper sleep

    let mut idle_start: Option<u64> = None;

    loop {
        let now = timer::current_time();

        // Check timers
        timer::check_timers();

        // Track how long we've been idle
        match idle_start {
            None => idle_start = Some(now),
            Some(start) => {
                let idle_time = now - start;

                if idle_time > IDLE_THRESHOLD_MS * 1_000_000 {
                    // Been idle for a while, enter deeper sleep
                    enter_deep_sleep();
                } else {
                    // Normal idle
                    enter_light_sleep();
                }
            }
        }

        // Reset idle tracking on wake
        if should_do_work() {
            idle_start = None;
        }
    }
}

/// Enter light sleep state (normal halt)
fn enter_light_sleep() {
    unsafe {
        crate::arch::enable_interrupts();
        crate::arch::halt();
        crate::arch::disable_interrupts();
    }
}

/// Enter deep sleep state (platform-specific power saving)
fn enter_deep_sleep() {
    // In real implementation, this would:
    // 1. Save additional CPU state
    // 2. Enter C3/C6 sleep state
    // 3. Wake on next interrupt
    // 4. Restore CPU state

    // For now, just do normal halt
    enter_light_sleep();
}

/// Check if there's work to do
fn should_do_work() -> bool {
    // This would check runqueues, pending interrupts, etc.
    // For now, always return false to stay in idle loop
    false
}

/// Per-CPU idle task data
#[derive(Debug)]
pub struct IdleTask {
    /// CPU this idle task runs on
    cpu: u32,
    /// Number of times idle loop has executed
    idle_count: u64,
    /// Total time spent idle (nanoseconds)
    idle_time: u64,
    /// Last time we entered idle
    last_idle_start: Option<u64>,
}

impl IdleTask {
    /// Create idle task data for a CPU
    pub fn new(cpu: u32) -> Self {
        Self {
            cpu,
            idle_count: 0,
            idle_time: 0,
            last_idle_start: None,
        }
    }

    /// Record entering idle state
    pub fn enter_idle(&mut self) {
        self.last_idle_start = Some(timer::current_time());
        self.idle_count += 1;
    }

    /// Record exiting idle state
    pub fn exit_idle(&mut self) {
        if let Some(start) = self.last_idle_start {
            let elapsed = timer::current_time() - start;
            self.idle_time += elapsed;
            self.last_idle_start = None;
        }
    }

    /// Get idle statistics
    pub fn stats(&self) -> IdleStats {
        IdleStats {
            cpu: self.cpu,
            idle_count: self.idle_count,
            total_idle_time: self.idle_time,
            currently_idle: self.last_idle_start.is_some(),
        }
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.idle_count = 0;
        self.idle_time = 0;
    }
}

/// Idle task statistics
#[derive(Debug, Clone, Copy)]
pub struct IdleStats {
    /// CPU ID
    pub cpu: u32,
    /// Number of idle iterations
    pub idle_count: u64,
    /// Total time spent idle (nanoseconds)
    pub total_idle_time: u64,
    /// Currently in idle state
    pub currently_idle: bool,
}

impl IdleStats {
    /// Calculate average idle time per iteration
    pub fn avg_idle_time_ns(&self) -> u64 {
        if self.idle_count > 0 {
            self.total_idle_time / self.idle_count
        } else {
            0
        }
    }

    /// Calculate idle percentage (would need total time as input)
    pub fn idle_percentage(&self, total_time_ns: u64) -> f64 {
        if total_time_ns > 0 {
            (self.total_idle_time as f64 / total_time_ns as f64) * 100.0
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_idle_task_stats() {
        let mut idle = IdleTask::new(0);

        assert_eq!(idle.stats().idle_count, 0);
        assert!(!idle.stats().currently_idle);

        idle.enter_idle();
        assert!(idle.stats().currently_idle);
        assert_eq!(idle.stats().idle_count, 1);

        // Simulate time passing (in real test, would use mocked time)
        idle.exit_idle();
        assert!(!idle.stats().currently_idle);
    }

    #[test]
    fn test_idle_stats_calculation() {
        let stats = IdleStats {
            cpu: 0,
            idle_count: 100,
            total_idle_time: 1_000_000_000, // 1 second
            currently_idle: false,
        };

        assert_eq!(stats.avg_idle_time_ns(), 10_000_000); // 10ms average
        assert_eq!(stats.idle_percentage(10_000_000_000), 10.0); // 10%
    }
}
