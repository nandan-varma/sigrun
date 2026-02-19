//! Task representation and management
//!
//! Provides Task, TaskContext, TaskId, TaskState, and related structures.

use core::sync::atomic::{AtomicU64, Ordering};

/// Unique task identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

impl TaskId {
    /// Create a new unique TaskId
    pub fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(1);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    /// Get the underlying u64 value
    pub fn as_u64(self) -> u64 {
        self.0
    }

    /// Create from a raw u64 (use with caution)
    pub const fn from_u64(id: u64) -> Self {
        Self(id)
    }

    /// Null/invalid task ID
    pub const fn null() -> Self {
        Self(0)
    }

    /// Check if this is the null task ID
    pub fn is_null(self) -> bool {
        self.0 == 0
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

/// Task priority (0 = highest, 255 = lowest)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Priority(u8);

impl Priority {
    /// Maximum priority (0)
    pub const MAX: Priority = Priority(0);
    /// Minimum priority (255)
    pub const MIN: Priority = Priority(255);
    /// Default priority (middle)
    pub const DEFAULT: Priority = Priority(128);
    /// Real-time priority (high)
    pub const REALTIME: Priority = Priority(16);
    /// Idle priority (lowest)
    pub const IDLE: Priority = Priority(255);

    /// Create a new priority
    pub fn new(p: u8) -> Self {
        Self(p)
    }

    /// Get the underlying u8 value
    pub fn as_u8(self) -> u8 {
        self.0
    }

    /// Check if this is a real-time priority
    pub fn is_realtime(self) -> bool {
        self.0 <= 31
    }

    /// Check if this is an idle priority
    pub fn is_idle(self) -> bool {
        self.0 >= 250
    }
}

impl Default for Priority {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Task state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Task is ready to run
    Ready,
    /// Task is currently running
    Running,
    /// Task is blocked waiting for something
    Blocked(BlockReason),
    /// Task is sleeping until a deadline
    Sleeping(Deadline),
    /// Task has terminated
    Terminated,
}

/// Reason a task is blocked
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockReason {
    /// Waiting for a mutex/lock
    Lock,
    /// Waiting for I/O
    Io,
    /// Waiting for a condition variable
    Condition,
    /// Waiting for a message (IPC)
    Ipc,
    /// Waiting for a signal
    Signal,
    /// Blocked for debugging
    Debug,
    /// Other reason
    Other(u64),
}

/// Deadline for sleeping tasks (nanoseconds since boot)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Deadline(u64);

impl Deadline {
    /// Create a deadline from nanoseconds
    pub fn from_ns(ns: u64) -> Self {
        Self(ns)
    }

    /// Get the underlying value
    pub fn as_ns(self) -> u64 {
        self.0
    }

    /// Check if this deadline has passed
    pub fn is_expired(self, now: u64) -> bool {
        now >= self.0
    }

    /// Time remaining until deadline (0 if expired)
    pub fn remaining(self, now: u64) -> u64 {
        if self.0 > now {
            self.0 - now
        } else {
            0
        }
    }
}

/// CPU affinity mask (for multi-core systems)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuAffinity(u64);

impl CpuAffinity {
    /// Allow task to run on any CPU
    pub const ANY: CpuAffinity = CpuAffinity(!0);

    /// Create affinity for a single CPU
    pub fn single(cpu: u32) -> Self {
        Self(1u64 << cpu)
    }

    /// Create affinity from a mask
    pub fn from_mask(mask: u64) -> Self {
        Self(mask)
    }

    /// Check if task can run on given CPU
    pub fn allows_cpu(self, cpu: u32) -> bool {
        (self.0 & (1u64 << cpu)) != 0
    }

    /// Get the mask
    pub fn mask(self) -> u64 {
        self.0
    }

    /// Add a CPU to the mask
    pub fn add_cpu(&mut self, cpu: u32) {
        self.0 |= 1u64 << cpu;
    }

    /// Remove a CPU from the mask
    pub fn remove_cpu(&mut self, cpu: u32) {
        self.0 &= !(1u64 << cpu);
    }
}

impl Default for CpuAffinity {
    fn default() -> Self {
        Self::ANY
    }
}

/// CPU ID type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CpuId(u32);

impl CpuId {
    /// Create a new CpuId
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Get the underlying value
    pub fn as_u32(self) -> u32 {
        self.0
    }
}

impl Default for CpuId {
    fn default() -> Self {
        Self(0)
    }
}

/// Task context - saved register state for context switching
///
/// On x86_64, we need to save:
/// - R15, R14, R13, R12 (callee-saved)
/// - RBP (base pointer)
/// - RBX (additional callee-saved)
/// - RSP (stack pointer) - stored in Task
/// - RIP (instruction pointer) - stored in Task
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct TaskContext {
    /// R15 register
    pub r15: u64,
    /// R14 register
    pub r14: u64,
    /// R13 register
    pub r13: u64,
    /// R12 register
    pub r12: u64,
    /// RBP register
    pub rbp: u64,
    /// RBX register
    pub rbx: u64,
    /// RFLAGS register
    pub rflags: u64,
}

impl TaskContext {
    /// Create a new empty context
    pub const fn new() -> Self {
        Self {
            r15: 0,
            r14: 0,
            r13: 0,
            r12: 0,
            rbp: 0,
            rbx: 0,
            rflags: 0x202, // IF flag set
        }
    }
}

/// Task statistics
#[derive(Debug, Clone, Copy, Default)]
pub struct TaskStats {
    /// Total CPU time used (nanoseconds)
    pub cpu_time: u64,
    /// Number of context switches
    pub context_switches: u64,
    /// Time when task was created
    pub created_at: u64,
    /// Time when task started running (if ever)
    pub started_at: Option<u64>,
}

/// Task structure - represents a schedulable unit
#[derive(Debug)]
pub struct Task {
    /// Unique task ID
    pub id: TaskId,
    /// Current state
    pub state: TaskState,
    /// Task priority
    pub priority: Priority,
    /// CPU this task is running on (or was last running)
    pub cpu: CpuId,
    /// CPU affinity mask
    pub affinity: CpuAffinity,
    /// Time slice remaining (nanoseconds)
    pub time_slice: u64,
    /// Kernel stack pointer (when not running)
    pub kernel_stack: u64,
    /// User stack pointer (for user tasks)
    pub user_stack: Option<u64>,
    /// Saved register context
    pub context: TaskContext,
    /// Instruction pointer (when not running)
    pub rip: u64,
    /// Address space ID
    pub address_space: u64,
    /// Wakeup time (for sleeping tasks)
    pub wakeup_time: Option<Deadline>,
    /// Task statistics
    pub stats: TaskStats,
    /// Entry point (for debugging)
    pub entry_point: u64,
}

impl Task {
    /// Create a new kernel task
    pub fn new_kernel(entry: fn() -> !, priority: Priority) -> Self {
        let id = TaskId::new();
        let stack_size = 8192; // 8KB kernel stack

        // In real implementation, allocate stack from kernel heap
        // For now, use a placeholder address
        let kernel_stack = 0xFFFF_8000_0000_0000 + (id.as_u64() * stack_size) + stack_size;

        Self {
            id,
            state: TaskState::Ready,
            priority,
            cpu: CpuId::default(),
            affinity: CpuAffinity::ANY,
            time_slice: 10_000_000, // 10ms default
            kernel_stack,
            user_stack: None,
            context: TaskContext::new(),
            rip: entry as u64,
            address_space: 0, // Kernel address space
            wakeup_time: None,
            stats: TaskStats {
                created_at: crate::timer::current_time(),
                ..TaskStats::default()
            },
            entry_point: entry as u64,
        }
    }

    /// Create the idle task
    pub fn new_idle() -> Self {
        Self {
            id: TaskId::from_u64(0),
            state: TaskState::Ready,
            priority: Priority::IDLE,
            cpu: CpuId::default(),
            affinity: CpuAffinity::ANY,
            time_slice: u64::MAX, // Idle task never times out
            kernel_stack: 0,
            user_stack: None,
            context: TaskContext::new(),
            rip: idle_loop as u64,
            address_space: 0,
            wakeup_time: None,
            stats: TaskStats::default(),
            entry_point: idle_loop as u64,
        }
    }

    /// Check if task is ready to run
    pub fn is_ready(&self) -> bool {
        matches!(self.state, TaskState::Ready)
    }

    /// Check if task is currently running
    pub fn is_running(&self) -> bool {
        matches!(self.state, TaskState::Running)
    }

    /// Check if task is blocked
    pub fn is_blocked(&self) -> bool {
        matches!(self.state, TaskState::Blocked(_))
    }

    /// Check if task has terminated
    pub fn is_terminated(&self) -> bool {
        matches!(self.state, TaskState::Terminated)
    }

    /// Can this task run on the given CPU?
    pub fn can_run_on(&self, cpu: u32) -> bool {
        self.affinity.allows_cpu(cpu)
    }

    /// Get the priority level (0-31, where 0 is highest)
    /// This maps the 0-255 priority to MLFQ levels
    pub fn priority_level(&self) -> usize {
        let p = self.priority.as_u8() as usize;
        // Map 0-255 to 0-31
        (p / 8).min(31)
    }
}

/// Idle task loop (does nothing but halt)
fn idle_loop() -> ! {
    loop {
        crate::arch::halt();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_id_generation() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
        assert!(id1.as_u64() > 0);
        assert!(id2.as_u64() > id1.as_u64());
    }

    #[test]
    fn test_priority_comparison() {
        let high = Priority::new(0);
        let low = Priority::new(255);
        assert!(high < low); // Higher priority = lower number
        assert!(high.is_realtime());
        assert!(!low.is_realtime());
        assert!(low.is_idle());
    }

    #[test]
    fn test_cpu_affinity() {
        let mut affinity = CpuAffinity::single(0);
        assert!(affinity.allows_cpu(0));
        assert!(!affinity.allows_cpu(1));

        affinity.add_cpu(1);
        assert!(affinity.allows_cpu(0));
        assert!(affinity.allows_cpu(1));

        affinity.remove_cpu(0);
        assert!(!affinity.allows_cpu(0));
        assert!(affinity.allows_cpu(1));
    }

    #[test]
    fn test_deadline() {
        let deadline = Deadline::from_ns(1000);
        assert!(deadline.is_expired(1000));
        assert!(deadline.is_expired(1001));
        assert!(!deadline.is_expired(999));
        assert_eq!(deadline.remaining(500), 500);
        assert_eq!(deadline.remaining(1000), 0);
    }
}
