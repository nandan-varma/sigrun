//! Preemptive round-robin scheduler with real context switching.
//!
//! The scheduler owns a static task table (64 slots).  On each 10 ms LAPIC
//! timer tick, `tick()` is called from the ISR; it selects the next ready
//! task and calls `switch_context` to perform a live CPU context switch.

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
        Self {
            slots: [const { None }; MAX_TASKS],
            len: 0,
            current: 0,
        }
    }

    fn add(&mut self, task: Task) -> Option<usize> {
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(task);
                self.len += 1;
                return Some(i);
            }
        }
        None
    }

    /// Round-robin selection: non-idle tasks first, then idle as fallback.
    fn next_ready(&mut self) -> Option<usize> {
        if self.len == 0 {
            return None;
        }
        let start = self.current;
        // Prefer non-idle tasks.
        for i in 0..MAX_TASKS {
            let idx = (start + i + 1) % MAX_TASKS;
            if let Some(ref t) = self.slots[idx] {
                if t.is_ready() && !t.priority.is_idle() {
                    self.current = idx;
                    return Some(idx);
                }
            }
        }
        // Fall back to idle.
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

/// Index of the currently-running task (usize::MAX = boot / no task).
static CURRENT_TASK_IDX: AtomicUsize = AtomicUsize::new(usize::MAX);

/// Index of the idle task (usize::MAX until registered).
pub static IDLE_TASK_IDX: AtomicUsize = AtomicUsize::new(usize::MAX);

/// Saved RSP for the boot/exit path (write-once, no need to restore).
static mut BOOT_SAVED_RSP: u64 = 0;

// ── Public scheduler config ───────────────────────────────────────────────────

pub struct SchedulerConfig {
    pub cpu_count: usize,
    pub time_slice_ns: u64,
}

impl Default for SchedulerConfig {
    fn default() -> Self {
        Self {
            cpu_count: 1,
            time_slice_ns: DEFAULT_TIME_SLICE_NS,
        }
    }
}

pub struct Scheduler {
    pub config: SchedulerConfig,
}

impl Scheduler {
    pub fn new(config: SchedulerConfig) -> Self {
        Self { config }
    }
    pub fn init() -> Self {
        Self::new(SchedulerConfig::default())
    }

    /// Spawn a kernel task.  Returns the task ID.
    pub fn spawn_kernel(&mut self, entry: fn() -> !, priority: Priority) -> TaskId {
        let task = Task::new_kernel(entry, priority);
        let id = task.id;
        let idx = TASK_TABLE.lock().add(task).expect("task table full");
        crate::log::fmt(format_args!(
            "[TASK ] spawned '{}' id={} pri={} stack={:#x}",
            "ktask",
            id.as_u64(),
            priority.as_u8(),
            TASK_TABLE.lock().slots[idx].as_ref().unwrap().kernel_stack
        ));
        id
    }

    /// Register the idle task and record its slot index.
    pub fn register_idle(&mut self) -> TaskId {
        let task = Task::new_idle();
        let id = task.id;
        let idx = TASK_TABLE
            .lock()
            .add(task)
            .expect("task table full for idle");
        IDLE_TASK_IDX.store(idx, Ordering::SeqCst);
        crate::log::fmt(format_args!(
            "[TASK ] idle     id={} pri=255 stack={:#x}",
            id.as_u64(),
            TASK_TABLE.lock().slots[idx].as_ref().unwrap().kernel_stack
        ));
        id
    }

    pub fn task_count(&self) -> usize {
        TASK_TABLE.lock().len
    }
}

// ── Module-level convenience functions ───────────────────────────────────────

pub fn init() -> Scheduler {
    Scheduler::init()
}

/// Spawn a kernel task outside of Scheduler struct (called from main).
pub fn spawn_kernel_task(name: &str, entry: fn() -> !, priority: Priority) -> TaskId {
    let task = Task::new_kernel(entry, priority);
    let id = task.id;
    let idx = TASK_TABLE.lock().add(task).expect("task table full");
    let stack = TASK_TABLE.lock().slots[idx].as_ref().unwrap().kernel_stack;
    crate::log::fmt(format_args!(
        "[TASK ] spawned '{}' id={} pri={} stack={:#x}",
        name,
        id.as_u64(),
        priority.as_u8(),
        stack
    ));
    id
}

/// Register the idle task and return its ID.
pub fn register_idle_task() -> TaskId {
    let task = Task::new_idle();
    let id = task.id;
    let idx = TASK_TABLE.lock().add(task).expect("task table full");
    IDLE_TASK_IDX.store(idx, Ordering::SeqCst);
    let stack = TASK_TABLE.lock().slots[idx].as_ref().unwrap().kernel_stack;
    crate::log::fmt(format_args!(
        "[TASK ] idle     id={} pri=255 stack={:#x}",
        id.as_u64(),
        stack
    ));
    id
}

/// Create initial process (legacy alias kept for backward compat).
pub fn create_init_process() -> TaskId {
    register_idle_task()
}

/// Enter the idle loop.  Called after all tasks are registered and the
/// ring-3 demo (if any) has finished.  Never returns.
pub fn enter_idle() -> ! {
    let idle_idx = IDLE_TASK_IDX.load(Ordering::Relaxed);
    CURRENT_TASK_IDX.store(idle_idx, Ordering::SeqCst);
    crate::log::info("[SCHED] Preemptive scheduler active");
    crate::arch::enable_interrupts();
    loop {
        crate::arch::halt();
    }
}

/// Original start() – enables interrupts and enters idle.  Never returns.
pub fn start() -> ! {
    enter_idle()
}

/// Called by the LAPIC timer ISR on every 10 ms tick.
///
/// Selects the next ready task and calls `switch_context` if a different task
/// should run.  Must be called with interrupts already disabled (inside ISR).
pub fn tick() {
    TICK_COUNT.fetch_add(1, Ordering::Relaxed);

    let mut table = TASK_TABLE.lock();
    let current_idx = CURRENT_TASK_IDX.load(Ordering::Relaxed);

    let next_idx = match table.next_ready() {
        Some(idx) => idx,
        None => return,
    };

    if next_idx == current_idx {
        return;
    }

    let next_rsp = match table.slots[next_idx].as_ref() {
        Some(t) => t.kernel_stack,
        None => return,
    };

    // Pointer to old task's saved RSP field (must outlive the lock).
    let old_rsp_ptr: *mut u64 = if current_idx == usize::MAX {
        // First switch from boot/exit context: discard old RSP.
        &raw mut BOOT_SAVED_RSP
    } else {
        match table.slots[current_idx].as_mut() {
            Some(t) => &mut t.kernel_stack,
            None => return,
        }
    };

    CURRENT_TASK_IDX.store(next_idx, Ordering::SeqCst);
    drop(table); // Release lock before switching stacks.

    unsafe { crate::arch::x86_64::switch::switch_context(old_rsp_ptr, next_rsp) }
}

/// Switch from syscall/exit context to the idle task (and then to kernel tasks
/// via normal timer-driven preemption).  Never returns.
pub fn switch_to_idle() -> ! {
    let idle_idx = IDLE_TASK_IDX.load(Ordering::Relaxed);
    let idle_rsp = {
        let table = TASK_TABLE.lock();
        table.slots[idle_idx]
            .as_ref()
            .map(|t| t.kernel_stack)
            .unwrap_or(0)
    };

    CURRENT_TASK_IDX.store(idle_idx, Ordering::SeqCst);

    // Use BOOT_SAVED_RSP as a throwaway old-rsp slot.
    unsafe { crate::arch::x86_64::switch::switch_context(&raw mut BOOT_SAVED_RSP, idle_rsp) }
    unreachable!()
}

pub fn tick_count() -> usize {
    TICK_COUNT.load(Ordering::Relaxed)
}

pub fn current_task_id() -> Option<TaskId> {
    let idx = CURRENT_TASK_IDX.load(Ordering::Relaxed);
    if idx == usize::MAX {
        return None;
    }
    TASK_TABLE.lock().slots[idx].as_ref().map(|t| t.id)
}

// ── Compat stubs ─────────────────────────────────────────────────────────────

pub struct SchedulerConfig2 {
    pub cpu_count: usize,
    pub load_balancing: bool,
    pub priority_boost: bool,
    pub time_slice_ns: u64,
    pub rt_reserve_percent: u8,
}
