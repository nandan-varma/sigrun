//! SIGRUN System Call API
//!
//! Low-level system call interface between userspace and kernel.

#![no_std]

use core::arch::asm;

pub mod arg;
pub mod error;
pub mod number;

/// Syscall numbers
pub use number::*;

/// Syscall argument wrapper
pub use arg::SyscallArgs;

/// Syscall error codes
pub use error::*;

/// Syscall result type
pub type SyscallResult = Result<u64, SyscallError>;

/// Perform a system call
///
/// This is a low-level interface. Most code should use the
/// typed wrappers in other modules.
///
/// # Safety
///
/// Traps directly into the kernel via the `syscall` instruction with
/// caller-supplied register values. The caller must ensure `args.num`
/// names a real syscall and that `args.arg0..arg5` are valid for
/// whatever that syscall expects (e.g. any pointer/length pair must
/// reference memory the caller actually owns) — the kernel handler on
/// the other side is trusted to validate inputs, but an unsupported or
/// malformed syscall number/argument combination is caller-defined
/// behavior, not guaranteed to fail gracefully.
#[inline]
pub unsafe fn syscall(args: SyscallArgs) -> Result<u64, SyscallError> {
    let result: u64;

    #[cfg(target_arch = "x86_64")]
    asm!(
        "syscall",
        in("rax") args.num,
        in("rdi") args.arg0,
        in("rsi") args.arg1,
        in("rdx") args.arg2,
        in("r10") args.arg3,
        in("r8") args.arg4,
        in("r9") args.arg5,
        lateout("rax") result,
        options(nomem, nostack, preserves_flags)
    );

    #[cfg(target_arch = "aarch64")]
    asm!(
        "svc #0",
        in("x8") args.num,
        in("x0") args.arg0,
        in("x1") args.arg1,
        in("x2") args.arg2,
        in("x3") args.arg3,
        in("x4") args.arg4,
        in("x5") args.arg5,
        lateout("x0") result,
        options(nomem, nostack)
    );

    // Check for error (negative return value)
    if result > 0xFFFFFFFFFFFFF000 {
        Err(SyscallError::from_raw(result as i64))
    } else {
        Ok(result)
    }
}
