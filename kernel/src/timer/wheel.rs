//! Timer wheel implementation - simplified version

use super::{current_time, TimerError, TimerId};

const MAX_TIMERS: usize = 64;

const NUM_WHEELS: usize = 4;
const WHEEL_BITS: [usize; NUM_WHEELS] = [8, 8, 8, 32];
const WHEEL_SIZE: usize = 64;

#[derive(Debug, Clone, Copy)]
struct WheelEntry {
    expiry_ns: u64,
    callback: fn(),
    id: TimerId,
}

impl WheelEntry {
    const fn empty() -> Self {
        Self {
            expiry_ns: u64::MAX,
            callback: || {},
            id: TimerId::from_u64(0),
        }
    }
}

pub struct TimerWheel {
    current_time: u64,
    wheels: [[WheelEntry; 8]; 4],
    overflow: [WheelEntry; 8],
    overflow_count: usize,
    active_count: usize,
}

impl TimerWheel {
    pub fn new() -> Self {
        let mut wheels = [[WheelEntry::empty(); 8]; 4];
        for wheel in wheels.iter_mut() {
            for entry in wheel.iter_mut() {
                *entry = WheelEntry::empty();
            }
        }
        Self {
            current_time: 0,
            wheels,
            overflow: [WheelEntry::empty(); 8],
            overflow_count: 0,
            active_count: 0,
        }
    }

    pub fn schedule(&mut self, delay_ns: u64, callback: fn()) -> TimerId {
        let now = self.current_time;
        let expiry = now.saturating_add(delay_ns);
        let id = TimerId::new();

        let entry = WheelEntry {
            expiry_ns: expiry,
            callback,
            id,
        };

        for level in 0..NUM_WHEELS {
            let granularity = match level {
                0 => 1_000_000,
                1 => 8_000_000,
                2 => 64_000_000,
                3 => 512_000_000,
                _ => 1_000_000_000,
            };
            let range = granularity * WHEEL_BITS[level];

            if expiry - now < range as u64 {
                let idx = ((expiry / granularity) & 7) as usize;
                self.wheels[level][idx] = entry;
                self.active_count += 1;
                return id;
            }
        }

        if self.overflow_count < 8 {
            self.overflow[self.overflow_count] = entry;
            self.overflow_count += 1;
            self.active_count += 1;
        }

        id
    }

    pub fn check_expired(&mut self, now: u64) {
        self.current_time = now;

        for level in 0..NUM_WHEELS {
            let granularity = match level {
                0 => 1_000_000,
                1 => 8_000_000,
                2 => 64_000_000,
                3 => 512_000_000,
                _ => 1_000_000_000,
            };
            let slot = ((now / granularity) & 7) as usize;

            let entries = &mut self.wheels[level][slot..];
            for entry in entries.iter_mut() {
                if entry.id.as_u64() != 0 && entry.expiry_ns <= now {
                    (entry.callback)();
                    *entry = WheelEntry::empty();
                    self.active_count -= 1;
                }
            }
        }
    }

    pub fn cancel(&mut self, id: TimerId) -> Result<(), TimerError> {
        if id.as_u64() == 0 {
            return Err(TimerError::InvalidId);
        }

        for level in 0..NUM_WHEELS {
            for entry in self.wheels[level].iter_mut() {
                if entry.id == id {
                    *entry = WheelEntry::empty();
                    self.active_count -= 1;
                    return Ok(());
                }
            }
        }

        for entry in self.overflow.iter_mut() {
            if entry.id == id {
                *entry = WheelEntry::empty();
                self.overflow_count -= 1;
                self.active_count -= 1;
                return Ok(());
            }
        }

        Err(TimerError::InvalidId)
    }

    pub fn active_count(&self) -> usize {
        self.active_count
    }
}

impl Default for TimerWheel {
    fn default() -> Self {
        Self::new()
    }
}

static mut WHEEL: Option<TimerWheel> = None;

pub fn init() {
    log::info!("    - Timer wheel initialized");
}

pub fn schedule(delay_ns: u64, callback: fn()) -> TimerId {
    unsafe {
        if let Some(ref mut wheel) = WHEEL {
            wheel.schedule(delay_ns, callback)
        } else {
            let mut wheel = TimerWheel::new();
            let id = wheel.schedule(delay_ns, callback);
            WHEEL = Some(wheel);
            id
        }
    }
}

pub fn cancel(id: TimerId) {
    unsafe {
        if let Some(ref mut wheel) = WHEEL {
            let _ = wheel.cancel(id);
        }
    }
}

pub fn check_expired(now: u64) {
    unsafe {
        if let Some(ref mut wheel) = WHEEL {
            wheel.check_expired(now);
        }
    }
}
