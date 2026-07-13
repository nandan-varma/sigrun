//! Stack-based kernel context switch for x86_64.
//!
//! A single `switch_context(old_rsp, new_rsp)` function saves callee-saved
//! registers + RFLAGS onto the current stack, stores RSP into `*old_rsp`,
//! loads `new_rsp`, and restores the next task's saved frame before returning
//! into that task's code path.

use core::arch::global_asm;

global_asm!(
    ".global switch_context",
    "switch_context:",
    // Save callee-saved regs + RFLAGS to the current (old) task's stack.
    "pushfq",
    "push r15",
    "push r14",
    "push r13",
    "push r12",
    "push rbp",
    "push rbx",
    // *old_rsp = rsp  (rdi = 1st arg, pointer to Task::kernel_stack)
    "mov qword ptr [rdi], rsp",
    // rsp = new_rsp  (rsi = 2nd arg, new task's saved kernel_stack)
    "mov rsp, rsi",
    // Restore next task's saved frame.
    "pop rbx",
    "pop rbp",
    "pop r12",
    "pop r13",
    "pop r14",
    "pop r15",
    "popfq",
    // Return address on the new stack is the task's code continuation.
    "ret",
);

extern "C" {
    /// Switch kernel stacks from the current task to the next.
    ///
    /// Saves {rbx,rbp,r12-r15,rflags} + stores RSP into `*old_rsp`, then
    /// restores the same frame from `new_rsp` and returns into the new task.
    ///
    /// # Safety
    /// - `old_rsp` must point to `Task::kernel_stack` of the current task.
    /// - `new_rsp` must be a valid kernel stack frame as produced by
    ///   `init_task_stack` or a prior `switch_context` call.
    /// - Interrupts should be disabled at the call site (they will be
    ///   restored by `popfq` for the new task).
    pub fn switch_context(old_rsp: *mut u64, new_rsp: u64);
}

/// Build the initial kernel stack frame for a fresh kernel task.
///
/// After `switch_context` switches to this task for the first time,
/// it will pop saved-registers (all zero), restore RFLAGS=0x202 (IF=1),
/// and `ret` into `entry_point`.
///
/// Returns the value to store in `Task::kernel_stack`.
///
/// # Safety
/// `stack_top` must be the exclusive-end (highest address) of an allocated
/// buffer that is at least 64 bytes and 16-byte aligned.
pub unsafe fn init_task_stack(stack_top: u64, entry_point: u64) -> u64 {
    // switch_context pop order: rbx, rbp, r12, r13, r14, r15, rflags, ret
    //   [rsp+0 ] rbx
    //   [rsp+8 ] rbp
    //   [rsp+16] r12
    //   [rsp+24] r13
    //   [rsp+32] r14
    //   [rsp+40] r15
    //   [rsp+48] rflags = 0x202 (IF=1, reserved bit 1)
    //   [rsp+56] entry_point   ← ret pops this into RIP
    let sp = stack_top as *mut u64;
    *sp.sub(1) = entry_point;
    *sp.sub(2) = 0x202;
    *sp.sub(3) = 0; // r15
    *sp.sub(4) = 0; // r14
    *sp.sub(5) = 0; // r13
    *sp.sub(6) = 0; // r12
    *sp.sub(7) = 0; // rbp
    *sp.sub(8) = 0; // rbx  ← rsp after switch_context saves its frame
    sp.sub(8) as u64
}

/// Build the initial kernel stack frame for a ring-3 (user-mode) task.
///
/// The first `switch_context` to this task restores registers, then
/// `ret`s to `ring3_trampoline` which executes `iretq` to enter user mode.
///
/// Returns the value to store in `Task::kernel_stack`.
///
/// # Safety
/// `kernel_stack_top` must be the exclusive-end of an allocated kernel stack
/// buffer (at least 128 bytes). `user_stack_top` and `user_rip` must be
/// valid user-mode addresses.
pub unsafe fn init_ring3_task_stack(
    kernel_stack_top: u64,
    user_stack_top: u64,
    user_rip: u64,
) -> u64 {
    // Layout from kernel_stack_top downward:
    //   IRETQ frame (5 qwords): SS, RSP(user), RFLAGS, CS, RIP(user)
    //   switch_context frame  : ret_addr=ring3_trampoline, rflags=0, r15..rbx
    let sp = kernel_stack_top as *mut u64;
    // IRETQ frame (CPU pops RIP first, then CS, RFLAGS, RSP, SS)
    *sp.sub(1) = 0x1b; // SS  = USER_DATA | 3
    *sp.sub(2) = user_stack_top; // RSP (user stack)
    *sp.sub(3) = 0x202; // RFLAGS (IF=1)
    *sp.sub(4) = 0x23; // CS  = USER_CODE | 3
    *sp.sub(5) = user_rip; // RIP (user entry)
                           // switch_context frame
    *sp.sub(6) = ring3_trampoline as u64; // ret addr → iretq
    *sp.sub(7) = 0; // rflags (IF=0 during kernel switch)
    *sp.sub(8) = 0; // r15
    *sp.sub(9) = 0; // r14
    *sp.sub(10) = 0; // r13
    *sp.sub(11) = 0; // r12
    *sp.sub(12) = 0; // rbp
    *sp.sub(13) = 0; // rbx  ← kernel_stack = sp.sub(13)
    sp.sub(13) as u64
}

// One-instruction trampoline: switch_context `ret`s here, then we IRETQ.
global_asm!(".global ring3_trampoline", "ring3_trampoline:", "iretq");
#[allow(dead_code)]
extern "C" {
    fn ring3_trampoline();
}
