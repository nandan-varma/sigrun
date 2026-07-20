//! Kernel syscall dispatch table.
//!
//! `rust_syscall_handler` is called by the assembly entry stub in
//! `arch::x86_64::syscall` with a pointer to the saved register frame.
//! We follow the Linux x86-64 ABI:
//!   rax = syscall number
//!   rdi, rsi, rdx, r10, r8, r9 = args 0-5
//! Return value in rax.

use crate::arch::x86_64::syscall::SyscallFrame;

// ── Syscall numbers (Linux-compatible subset) ─────────────────────────────────
pub const SYS_READ: u64 = 0;
pub const SYS_WRITE: u64 = 1;
pub const SYS_EXIT: u64 = 60;
pub const SYS_GETPID: u64 = 39;
pub const SYS_SCHED_YIELD: u64 = 24;
pub const SYS_NANOSLEEP: u64 = 35;

// ── Entry point called from assembly ─────────────────────────────────────────

#[no_mangle]
pub extern "C" fn rust_syscall_handler(frame: *mut SyscallFrame) -> u64 {
    let frame = unsafe { &mut *frame };
    let nr = frame.rax;
    let arg0 = frame.rdi;
    let arg1 = frame.rsi;
    let arg2 = frame.rdx;
    let _arg3 = frame.r10;
    let _arg4 = frame.r8;
    let _arg5 = frame.r9;

    match nr {
        SYS_WRITE => sys_write(arg0, arg1, arg2),
        SYS_GETPID => sys_getpid(),
        SYS_SCHED_YIELD => {
            sys_yield();
            0
        }
        SYS_NANOSLEEP => {
            sys_nanosleep(arg0, arg1);
            0
        }
        SYS_EXIT => sys_exit(arg0),
        _ => {
            crate::log::fmt(format_args!("[SYSCALL] unknown nr={}", nr));
            u64::MAX // ENOSYS
        }
    }
}

// ── Syscall implementations ───────────────────────────────────────────────────

/// sys_write: fd=1 (stdout) writes bytes to the serial console.
fn sys_write(fd: u64, buf_ptr: u64, len: u64) -> u64 {
    if fd != 1 && fd != 2 {
        return u64::MAX; // EBADF
    }
    if len == 0 {
        return 0;
    }
    // Safety: user-mode ptr is accessible because we set the U/S bit on all
    // identity-mapped pages.  In a real kernel this would be validated.
    let bytes = unsafe { core::slice::from_raw_parts(buf_ptr as *const u8, len as usize) };
    if let Ok(s) = core::str::from_utf8(bytes) {
        crate::arch::x86_64::serial::write(s);
    }
    len
}

/// sys_getpid: return the current task's ID.
fn sys_getpid() -> u64 {
    crate::scheduler::current_task_id()
        .map(|id| id.as_u64())
        .unwrap_or(0)
}

/// sys_sched_yield: voluntarily relinquish the CPU.
fn sys_yield() {
    crate::scheduler::tick();
}

/// sys_nanosleep: busy-wait for `timespec { tv_sec, tv_nsec }` at `req_ptr`.
fn sys_nanosleep(req_ptr: u64, _rem_ptr: u64) {
    if req_ptr == 0 {
        return;
    }
    let (secs, nsecs) = unsafe {
        let p = req_ptr as *const u64;
        (*p, *p.add(1))
    };
    let total_ns = secs * 1_000_000_000 + nsecs;
    crate::timer::sleep_ns(total_ns);
}

/// sys_exit: terminate the calling process and switch to the idle/next task.
fn sys_exit(code: u64) -> u64 {
    crate::log::fmt(format_args!("[USER ] exit({}) → kernel", code));
    // Switch to idle task; the timer-driven scheduler will resume kernel tasks.
    crate::scheduler::switch_to_idle();
}
