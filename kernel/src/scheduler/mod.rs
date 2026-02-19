//! Scheduler subsystem - simplified stub

mod runqueue;
mod task;

pub use runqueue::{Runqueue, SchedulerState, NUM_PRIORITY_LEVELS};
pub use task::{
    BlockReason, CpuAffinity, CpuId, Deadline, Priority, Task, TaskContext, TaskId, TaskState,
    TaskStats,
};

pub const DEFAULT_TIME_SLICE_NS: u64 = 10_000_000;

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
            load_balancing: true,
            priority_boost: true,
            time_slice_ns: DEFAULT_TIME_SLICE_NS,
            rt_reserve_percent: 20,
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

    pub fn add_task(&mut self, _task: Task) -> TaskId {
        TaskId::new()
    }

    pub fn schedule(&mut self, _cpu: usize) -> Option<TaskId> {
        None
    }

    pub fn create_init_process(&mut self) -> TaskId {
        log::info!("    Creating PID 1 (init)");
        TaskId::new()
    }

    pub fn start(&self) -> ! {
        log::info!("    Scheduler running on CPU 0");
        loop {
            crate::arch::halt();
        }
    }

    pub fn tick(&mut self, _cpu: usize) {}
    pub fn block(&mut self, _task_id: TaskId, _reason: BlockReason) {}
    pub fn wake(&mut self, _task_id: TaskId) {}
    pub fn sleep_until(&mut self, _task_id: TaskId, _deadline: Deadline) {}
    pub fn current_task(&self, _cpu: usize) -> Option<TaskId> {
        None
    }
    pub fn has_task(&self, _task_id: TaskId) -> bool {
        false
    }
    pub fn get_task(&self, _task_id: TaskId) -> Option<&Task> {
        None
    }
    pub fn task_count(&self) -> usize {
        0
    }
    pub fn set_priority(&mut self, _task_id: TaskId, _priority: Priority) -> bool {
        false
    }
}
