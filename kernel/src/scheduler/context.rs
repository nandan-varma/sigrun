//! Context switching implementation
//!
//! Handles saving and restoring task context for task switching.

use super::{Task, TaskContext};
use crate::arch::halt;

/// Context switcher - manages CPU context switching
pub struct ContextSwitcher;

impl ContextSwitcher {
    /// Create a new context switcher
    pub fn new() -> Self {
        Self
    }

    /// Switch from current task to next task
    ///
    /// # Safety
    /// This function is unsafe because it modifies CPU registers directly.
    /// It should only be called from scheduler code with interrupts disabled.
    pub unsafe fn switch(&self, current: &mut Task, next: &mut Task) {
        // Save current context
        let current_rsp = save_context(&mut current.context);
        current.kernel_stack = current_rsp;

        // Update stats
        current.stats.context_switches += 1;

        // Restore next context
        restore_context(&next.context, next.kernel_stack);

        // Note: after restore_context, we never return here
        // Instead, we resume at the return address saved in the context
    }

    /// Perform a context switch between two tasks
    /// This is the safe wrapper around the unsafe switch operation
    pub fn context_switch(&self, current: &mut Task, next: &mut Task) {
        // Disable interrupts during context switch
        crate::arch::disable_interrupts();

        unsafe {
            self.switch(current, next);
        }

        // Re-enable interrupts
        crate::arch::enable_interrupts();
    }

    /// Initialize a new task's context for first run
    pub fn init_task_context(task: &mut Task, entry_point: usize, stack_top: usize) {
        task.context = TaskContext::new();
        task.rip = entry_point as u64;
        task.kernel_stack = stack_top as u64;

        // Set up initial stack frame
        // On x86_64, when we "return" to a new task, the stack should look like
        // a normal function call return address
        unsafe {
            let stack = stack_top as *mut u64;
            // Push return address (entry point)
            *stack.sub(1) = entry_point as u64;
            // Push initial RFLAGS
            *stack.sub(2) = 0x202; // IF flag set
            task.kernel_stack = (stack.sub(2)) as u64;
        }
    }
}

impl Default for ContextSwitcher {
    fn default() -> Self {
        Self::new()
    }
}

/// Save current CPU context to the provided TaskContext
/// Returns the current RSP value
///
/// # Safety
/// This is a low-level assembly operation that modifies registers
#[cfg(target_arch = "x86_64")]
unsafe fn save_context(context: &mut TaskContext) -> u64 {
    let rsp: u64;

    core::arch::asm!(
        // Save callee-saved registers
        "mov [rdi + 0], r15",   // context.r15
        "mov [rdi + 8], r14",   // context.r14
        "mov [rdi + 16], r13",  // context.r13
        "mov [rdi + 24], r12",  // context.r12
        "mov [rdi + 32], rbp",  // context.rbp
        "mov [rdi + 40], rbx",  // context.rbx

        // Save RFLAGS
        "pushfq",
        "pop qword ptr [rdi + 48]", // context.rflags

        // Return current RSP
        "mov {}, rsp",

        lateout(reg) rsp,
        in("rdi") context as *mut TaskContext,
        options(nomem, preserves_flags)
    );

    rsp
}

/// Restore CPU context from the provided TaskContext and switch to new stack
///
/// # Safety
/// This function never returns - it jumps to the restored context
#[cfg(target_arch = "x86_64")]
unsafe fn restore_context(context: &TaskContext, new_stack: u64) -> ! {
    core::arch::asm!(
        // Switch to new stack
        "mov rsp, {stack}",

        // Restore callee-saved registers
        "mov r15, [rdi + 0]",   // context.r15
        "mov r14, [rdi + 8]",   // context.r14
        "mov r13, [rdi + 16]",  // context.r13
        "mov r12, [rdi + 24]",  // context.r12
        "mov rbp, [rdi + 32]",  // context.rbp
        "mov rbx, [rdi + 40]",  // context.rbx

        // Restore RFLAGS
        "push qword ptr [rdi + 48]", // context.rflags
        "popfq",

        // Return to task
        "ret",

        stack = in(reg) new_stack,
        in("rdi") context as *const TaskContext,
        options(noreturn, nomem)
    );
}

/// Stub implementations for non-x86_64 targets
#[cfg(not(target_arch = "x86_64"))]
unsafe fn save_context(_context: &mut TaskContext) -> u64 {
    0
}

#[cfg(not(target_arch = "x86_64"))]
unsafe fn restore_context(_context: &TaskContext, _new_stack: u64) -> ! {
    halt();
}

/// Assembly stub for task entry
/// This is the initial entry point for newly created tasks
#[cfg(target_arch = "x86_64")]
extern "C" fn task_entry() -> ! {
    // Enable interrupts
    crate::arch::enable_interrupts();

    // Get the task entry point and call it
    // In real implementation, this would call the actual task function

    loop {
        halt();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_switcher_creation() {
        let switcher = ContextSwitcher::new();
        // Just test that it can be created
    }

    #[test]
    fn test_init_task_context() {
        use crate::scheduler::{CpuAffinity, CpuId, Priority, Task, TaskId, TaskState, TaskStats};

        let mut task = Task {
            id: TaskId::new(),
            state: TaskState::Ready,
            priority: Priority::DEFAULT,
            cpu: CpuId::default(),
            affinity: CpuAffinity::ANY,
            time_slice: 0,
            kernel_stack: 0,
            user_stack: None,
            context: TaskContext::new(),
            rip: 0,
            address_space: 0,
            wakeup_time: None,
            stats: TaskStats::default(),
            entry_point: 0x1000,
        };

        ContextSwitcher::init_task_context(&mut task, 0x1000, 0xFFFF_8000_0000_1000);

        assert_eq!(task.rip, 0x1000);
        assert!(task.kernel_stack > 0);
    }
}
