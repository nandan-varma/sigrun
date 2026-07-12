//! Timer subsystem
//!
//! Provides timekeeping, timer interrupts, and scheduled callbacks.

mod clock;
mod hpet;
mod lapic;
mod wheel;

pub use clock::{ClockSource, Duration, SystemClock, Timestamp};
pub use wheel::TimerWheel;

use core::sync::atomic::{AtomicU64, Ordering};
static CURRENT_TIME_NS: AtomicU64 = AtomicU64::new(0);

pub fn init() {
    crate::log::info("  Initializing timer subsystem");
    clock::init();
    wheel::init();
    hpet::init();
    lapic::init();
    crate::log::info("  Timer subsystem ready");
}

/// Called from the LAPIC timer ISR (vector 32) after EOI.
pub fn on_tick() {
    advance_time_10ms();
    wheel::check_expired(current_time());
    crate::scheduler::tick();
}

/// Advance the kernel time counter by one 10 ms tick.
pub fn advance_time_10ms() {
    CURRENT_TIME_NS.fetch_add(10_000_000, core::sync::atomic::Ordering::Relaxed);
}

pub fn current_time() -> u64 {
    CURRENT_TIME_NS.load(Ordering::Relaxed)
}

pub fn update_time(time_ns: u64) {
    CURRENT_TIME_NS.store(time_ns, Ordering::Relaxed);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimerId(u64);

impl TimerId {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    pub fn as_u64(self) -> u64 {
        self.0
    }
    pub const fn from_u64(id: u64) -> Self {
        Self(id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerError {
    InvalidId,
    AlreadyExpired,
    WheelFull,
}

impl core::fmt::Display for TimerError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidId => write!(f, "Invalid timer ID"),
            Self::AlreadyExpired => write!(f, "Timer already expired"),
            Self::WheelFull => write!(f, "Timer wheel full"),
        }
    }
}

pub fn schedule_callback(delay_ns: u64, callback: fn()) -> TimerId {
    wheel::schedule(delay_ns, callback)
}

pub fn schedule_at(deadline_ns: u64, callback: fn()) -> TimerId {
    let now = current_time();
    if deadline_ns > now {
        schedule_callback(deadline_ns - now, callback)
    } else {
        schedule_callback(0, callback)
    }
}

pub fn cancel_callback(id: TimerId) {
    wheel::cancel(id);
}

pub fn check_timers() {
    wheel::check_expired(current_time());
}

pub fn sleep_ns(duration_ns: u64) {
    let deadline = current_time() + duration_ns;
    while current_time() < deadline {
        core::hint::spin_loop();
    }
}

pub fn busy_wait_us(us: u64) {
    for _ in 0..(us * 100) {
        core::hint::spin_loop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_id_generation() {
        let id1 = TimerId::new();
        let id2 = TimerId::new();
        assert_ne!(id1, id2);
    }
}
