//! Multi-Level Feedback Queue (MLFQ) runqueue implementation - simplified

use super::{CpuId, Priority, TaskId};

pub const NUM_PRIORITY_LEVELS: usize = 32;
const MAX_TASKS_PER_QUEUE: usize = 16;

type PriorityQueue = [Option<TaskId>; MAX_TASKS_PER_QUEUE];

fn empty_queue() -> PriorityQueue {
    [None; MAX_TASKS_PER_QUEUE]
}

pub struct Runqueue {
    pub cpu: CpuId,
    pub queues: [PriorityQueue; NUM_PRIORITY_LEVELS],
    pub current: Option<TaskId>,
    pub idle_task: TaskId,
    current_level: usize,
    task_count: usize,
    queue_heads: [usize; NUM_PRIORITY_LEVELS],
}

impl Runqueue {
    pub fn new(cpu: CpuId, idle_task: TaskId) -> Self {
        let mut queues = [empty_queue(); NUM_PRIORITY_LEVELS];
        for q in queues.iter_mut() {
            *q = empty_queue();
        }
        Self {
            cpu,
            queues,
            current: None,
            idle_task,
            current_level: 0,
            task_count: 0,
            queue_heads: [0; NUM_PRIORITY_LEVELS],
        }
    }

    pub fn enqueue(&mut self, task: TaskId, priority: Priority) {
        let level = priority_to_level(priority);
        let head = self.queue_heads[level];

        for i in 0..MAX_TASKS_PER_QUEUE {
            let idx = (head + i) % MAX_TASKS_PER_QUEUE;
            if self.queues[level][idx].is_none() {
                self.queues[level][idx] = Some(task);
                self.task_count += 1;
                return;
            }
        }
    }

    pub fn dequeue(&mut self) -> Option<TaskId> {
        for level in 0..NUM_PRIORITY_LEVELS {
            let head = self.queue_heads[level];
            for i in 0..MAX_TASKS_PER_QUEUE {
                let idx = (head + i) % MAX_TASKS_PER_QUEUE;
                if let Some(task) = self.queues[level][idx].take() {
                    self.queue_heads[level] = (idx + 1) % MAX_TASKS_PER_QUEUE;
                    self.current = Some(task);
                    self.current_level = level;
                    self.task_count -= 1;
                    return Some(task);
                }
            }
        }
        self.current = Some(self.idle_task);
        Some(self.idle_task)
    }

    pub fn load(&self) -> usize {
        self.task_count
    }
    pub fn current_task(&self) -> Option<TaskId> {
        self.current
    }
    pub fn is_empty(&self) -> bool {
        self.task_count == 0
    }

    pub fn boost_priorities(&mut self) {
        // Simplified: just reset heads
        for level in 1..NUM_PRIORITY_LEVELS {
            self.queue_heads[level] = 0;
            for q in self.queues[level].iter_mut() {
                *q = None;
            }
        }
    }

    pub fn stats(&self) -> RunqueueStats {
        let mut per_level = [0usize; NUM_PRIORITY_LEVELS];
        for (i, queue) in self.queues.iter().enumerate() {
            per_level[i] = queue.iter().filter(|o| o.is_some()).count();
        }

        RunqueueStats {
            cpu: self.cpu.as_u32(),
            total_tasks: self.task_count,
            current_task: self.current,
            per_level,
        }
    }
}

fn priority_to_level(priority: Priority) -> usize {
    let p = priority.as_u8() as usize;
    (p / 8).min(NUM_PRIORITY_LEVELS - 1)
}

#[derive(Debug, Clone)]
pub struct RunqueueStats {
    pub cpu: u32,
    pub total_tasks: usize,
    pub current_task: Option<TaskId>,
    pub per_level: [usize; NUM_PRIORITY_LEVELS],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulerState {
    Stopped,
    Starting,
    Running,
    Stopping,
}

impl Default for SchedulerState {
    fn default() -> Self {
        Self::Stopped
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runqueue_enqueue_dequeue() {
        let idle = TaskId::from_u64(0);
        let mut rq = Runqueue::new(CpuId::new(0), idle);

        let task1 = TaskId::from_u64(1);
        let task2 = TaskId::from_u64(2);

        rq.enqueue(task1, Priority::DEFAULT);
        rq.enqueue(task2, Priority::new(0));

        assert_eq!(rq.load(), 2);

        let next = rq.dequeue().unwrap();
        assert_eq!(next, task2);
    }
}
