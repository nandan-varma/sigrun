//! Clock source abstraction

pub trait ClockSource: Send + Sync {
    fn now(&self) -> Timestamp;
    fn resolution(&self) -> Duration;
    fn name(&self) -> &'static str;
    fn is_available(&self) -> bool;
}

pub struct SystemClock {
    primary: Option<&'static dyn ClockSource>,
}

impl SystemClock {
    pub fn new() -> Self {
        Self { primary: None }
    }

    pub fn now(&self) -> Timestamp {
        if let Some(p) = self.primary {
            p.now()
        } else {
            Timestamp { nanoseconds: 0 }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Timestamp {
    pub nanoseconds: u64,
}

impl Timestamp {
    pub fn new(ns: u64) -> Self {
        Self { nanoseconds: ns }
    }
    pub fn as_ns(self) -> u64 {
        self.nanoseconds
    }
    pub fn as_ms(self) -> u64 {
        self.nanoseconds / 1_000_000
    }
    pub fn as_secs(self) -> u64 {
        self.nanoseconds / 1_000_000_000
    }
    pub fn add(self, duration: Duration) -> Timestamp {
        Timestamp::new(self.nanoseconds.saturating_add(duration.nanoseconds))
    }
    pub fn sub(self, duration: Duration) -> Timestamp {
        Timestamp::new(self.nanoseconds.saturating_sub(duration.nanoseconds))
    }
}

impl Default for Timestamp {
    fn default() -> Self {
        Self { nanoseconds: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Duration {
    nanoseconds: u64,
}

impl Duration {
    pub fn new(ns: u64) -> Self {
        Self { nanoseconds: ns }
    }
    pub fn from_us(us: u64) -> Self {
        Self {
            nanoseconds: us * 1_000,
        }
    }
    pub fn from_ms(ms: u64) -> Self {
        Self {
            nanoseconds: ms * 1_000_000,
        }
    }
    pub fn from_secs(secs: u64) -> Self {
        Self {
            nanoseconds: secs * 1_000_000_000,
        }
    }
    pub fn as_ns(self) -> u64 {
        self.nanoseconds
    }
    pub fn as_us(self) -> u64 {
        self.nanoseconds / 1_000
    }
    pub fn as_ms(self) -> u64 {
        self.nanoseconds / 1_000_000
    }
    pub fn as_secs(self) -> u64 {
        self.nanoseconds / 1_000_000_000
    }
}

pub struct MonotonicClock {
    ticks: u64,
}

impl MonotonicClock {
    pub fn new() -> Self {
        Self { ticks: 0 }
    }
}

impl ClockSource for MonotonicClock {
    fn now(&self) -> Timestamp {
        Timestamp::new(self.ticks * 100)
    }
    fn resolution(&self) -> Duration {
        Duration::new(100)
    }
    fn name(&self) -> &'static str {
        "monotonic"
    }
    fn is_available(&self) -> bool {
        true
    }
}

impl Default for MonotonicClock {
    fn default() -> Self {
        Self::new()
    }
}

pub fn init() {
    crate::log::info_formatted("    - Clock subsystem initialized");
}
