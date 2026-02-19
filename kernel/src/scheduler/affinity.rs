//! CPU affinity management - simplified

use super::{CpuAffinity, CpuId, Task};

pub struct AffinityManager {
    cpu_count: u32,
}

impl AffinityManager {
    pub fn new(cpu_count: u32) -> Self {
        Self { cpu_count }
    }

    pub fn select_cpu(&self, task: &Task, cpu_loads: &[usize]) -> CpuId {
        if *task.affinity != CpuAffinity::ANY {
            return self.select_from_affinity(&task.affinity, cpu_loads);
        }
        self.select_least_loaded(cpu_loads)
    }

    fn select_from_affinity(&self, affinity: &CpuAffinity, cpu_loads: &[usize]) -> CpuId {
        let mut best_cpu = 0u32;
        let mut best_load = usize::MAX;

        for cpu in 0..self.cpu_count {
            if affinity.allows_cpu(cpu) {
                let load = cpu_loads.get(cpu as usize).copied().unwrap_or(0);
                if load < best_load {
                    best_load = load;
                    best_cpu = cpu;
                }
            }
        }

        CpuId::new(best_cpu)
    }

    fn select_least_loaded(&self, cpu_loads: &[usize]) -> CpuId {
        let mut best_cpu = 0u32;
        let mut best_load = usize::MAX;

        for (cpu, &load) in cpu_loads.iter().enumerate() {
            if load < best_load {
                best_load = load;
                best_cpu = cpu as u32;
            }
        }

        CpuId::new(best_cpu)
    }

    pub fn needs_rebalancing(&self, cpu_loads: &[usize]) -> bool {
        if cpu_loads.len() < 2 {
            return false;
        }

        let min = cpu_loads.iter().min().copied().unwrap_or(0);
        let max = cpu_loads.iter().max().copied().unwrap_or(0);

        max - min > 2
    }

    pub fn imbalance_factor(&self, cpu_loads: &[usize]) -> usize {
        if cpu_loads.is_empty() {
            return 0;
        }

        let total: usize = cpu_loads.iter().sum();
        let avg = total / cpu_loads.len();
        let max = cpu_loads.iter().max().copied().unwrap_or(0);

        if max > avg {
            max - avg
        } else {
            0
        }
    }
}

impl Default for AffinityManager {
    fn default() -> Self {
        Self::new(1)
    }
}

#[derive(Debug, Clone)]
pub struct CpuTopology {
    pub physical_cores: u32,
    pub logical_cores: u32,
    pub numa_nodes: u32,
    pub cpu_to_numa: [u32; 8],
}

impl CpuTopology {
    pub fn smp(cores: u32) -> Self {
        Self {
            physical_cores: cores,
            logical_cores: cores,
            numa_nodes: 1,
            cpu_to_numa: [0; 8],
        }
    }

    pub fn share_cache(&self, cpu1: u32, cpu2: u32) -> bool {
        cpu1 == cpu2
    }

    pub fn numa_node(&self, cpu: u32) -> u32 {
        if cpu < 8 {
            self.cpu_to_numa[cpu as usize]
        } else {
            0
        }
    }
}
