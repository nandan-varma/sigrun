//! SIGRUN scheduler – simple round-robin over a flat task table.
//!
//! The scheduler owns a static array of `Option<Task>` slots.  On each timer
//! tick (`tick()`) it advances the current-task pointer so the next task gets
//! CPU time.  Context switching is not yet implemented (no `context.rs` call),
//! so only one task runs at a time (the idle loop), but the infrastructure is
//! in place for real switching.

mod runqueue;
mod task;

pub use runqueue::{Runqueue, SchedulerState, NUM_PRIORITY_LEVELS};
pub use task::{
    BlockReason, CpuAffinity, CpuId, Deadline, Priority, Task, TaskContext, TaskId, TaskState,
    TaskStats,
};

use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

pub const DEFAULT_TIME_SLICE_NS: u64 = 10_000_000; // 10 ms
const MAX_TASKS: usize = 64;

// ── Task table ────────────────────────────────────────────────────────────────

struct TaskTable {
    slots: [Option<Task>; MAX_TASKS],
    len: usize,
    current: usize,
}

impl TaskTable {
    const fn new() -> Self {
        // SAFETY: Option<Task> is None by default; this is sound for a const context
        // because Task implements no Drop that the const evaluator would reject.
        Self {
            slots: [const { None }; MAX_TASKS],
            len: 0,
            current: 0,
        }
    }

    fn add(&mut self, task: Task) -> Option<TaskId> {
        for slot in self.slots.iter_mut() {
            if slot.is_none() {
                let id = task.id;
                *slot = Some(task);
                self.len += 1;
                return Some(id);
            }
        }
        None // table full
    }

    fn next_ready(&mut self) -> Option<usize> {
        if self.len == 0 {
            return None;
        }
        let start = self.current;
        for i in 0..MAX_TASKS {
            let idx = (start + i + 1) % MAX_TASKS;
            if let Some(ref t) = self.slots[idx] {
                if t.is_ready() {
                    self.current = idx;
                    return Some(idx);
                }
            }
        }
        None
    }
}

static TASK_TABLE: Mutex<TaskTable> = Mutex::new(TaskTable::new());
static TICK_COUNT: AtomicUsize = AtomicUsize::new(0);

// ── Scheduler config / public struct (kept for ABI compatibility) ─────────────

pub struct SchedulerConfig {
    pub cpu_count: usize,
    pub load_balancing: bool,
    pub priority_boost: bool,
    pub time_slice_ns: u64,
    pub rt_reserve_percent: u8,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            cpu_count: 1,
            load_balancing: false,
            priority_boost: false,
            time_slice_ns: DEFAULT_TIME_SLICE_NS,
            rt_reserve_percent: 0,
        }
    }
}

pub struct Scheduler {
    config: SchedulerConfig,
}

impl Scheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self { config }
    }

    pub fn init() -> Self {
        Self::new(SchedulerConfig::default())
    }

    pub fn cpu_count(&self) -> usize {
        self.config.cpu_count
    }

    pub fn add_task(&mut self, task: Task) -> TaskId {
        let id = task.id;
        TASK_TABLE.lock().add(task);
        id
    }

    pub fn schedule(&mut self, _cpu: usize) -> Option<TaskId> {
        let mut table = TASK_TABLE.lock();
        let idx = table.next_ready()?;
        table.slots[idx].as_ref().map(|t| t.id)
    }

    pub fn create_init_process(&mut self) -> TaskId {
        crate::log::info("  Creating init task (PID 1)");
        let task = Task::new_idle(); // init shares the idle loop for now
        let id = task.id;
        TASK_TABLE.lock().add(task);
        id
    }

    pub fn start(&self) -> ! {
        crate::log::info("SIGRUN kernel scheduler started – entering idle loop");
        crate::arch::enable_interrupts();
        crate::arch::halt()
    }

    pub fn tick(&mut self, _cpu: usize) {
        TICK_COUNT.fetch_add(1, Ordering::Relaxed);
        TASK_TABLE.lock().next_ready();
    }

    pub fn block(&mut self, _task_id: TaskId, _reason: BlockReason) {}
    pub fn wake(&mut self, _task_id: TaskId) {}
    pub fn sleep_until(&mut self, _task_id: TaskId, _deadline: Deadline) {}
    pub fn current_task(&self, _cpu: usize) -> Option<TaskId> { None }
    pub fn has_task(&self, task_id: TaskId) -> bool {
        TASK_TABLE.lock().slots.iter().any(|s| {
            s.as_ref().map(|t| t.id == task_id).unwrap_or(false)
        })
    }
    pub fn task_count(&self) -> usize {
        TASK_TABLE.lock().len
    }
    pub fn set_priority(&mut self, _task_id: TaskId, _priority: Priority) -> bool { false }
}

// ── Module-level convenience functions ───────────────────────────────────────

/// Initialise the scheduler and return a handle.
pub fn init() -> Scheduler {
    Scheduler::init()
}

/// Create the initial userspace process (PID 1).
pub fn create_init_process() -> TaskId {
    let task = Task::new_idle();
    let id = task.id;
    TASK_TABLE.lock().add(task);
    crate::log::info("  Init task registered");
    id
}

/// Called once the rest of the kernel is initialised; enters the idle loop.
pub fn start() -> ! {
    crate::log::info("SIGRUN kernel running");
    crate::arch::enable_interrupts();
    crate::arch::halt()
}

/// Called by the timer ISR on each 10 ms tick.
pub fn tick() {
    TICK_COUNT.fetch_add(1, Ordering::Relaxed);
}
