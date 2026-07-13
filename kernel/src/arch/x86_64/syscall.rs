//! SYSCALL/SYSRET fast-path for user→kernel transitions.
//!
//! Registers LSTAR with a hand-written entry stub that switches to a
//! dedicated kernel stack, builds a `SyscallFrame` on it, and calls
//! `rust_syscall_handler` (defined in `crate::syscall`).

use core::arch::global_asm;

// ── MSR addresses ─────────────────────────────────────────────────────────────
const MSR_EFER: u32 = 0xC000_0080;
const MSR_STAR: u32 = 0xC000_0081;
const MSR_LSTAR: u32 = 0xC000_0082;
const MSR_SFMASK: u32 = 0xC000_0084;
const EFER_SCE: u64 = 1; // SYSCALL Enable bit

// ── Dedicated syscall kernel stack ────────────────────────────────────────────
const SYSCALL_STACK_SIZE: usize = 8192;
#[repr(align(16))]
struct SyscallStack([u8; SYSCALL_STACK_SIZE]);
static mut SYSCALL_STACK_BUF: SyscallStack = SyscallStack([0; SYSCALL_STACK_SIZE]);

// These are referenced by name from the global_asm! stub.
#[no_mangle]
static mut sigrun_syscall_user_rsp: u64 = 0;
#[no_mangle]
static mut sigrun_syscall_kernel_rsp: u64 = 0;

// ── SYSCALL entry stub ────────────────────────────────────────────────────────
//
// On SYSCALL the CPU has:
//   RCX = saved user RIP
//   R11 = saved user RFLAGS
//   CS/SS set from STAR
//   IF cleared by SFMASK
//   RSP still points at user stack!
//
// We build a SyscallFrame on the dedicated kernel stack and call
// rust_syscall_handler(frame_ptr).  On return rax holds the result.
global_asm!(
    ".global syscall_entry",
    "syscall_entry:",
    // Save user RSP and switch to kernel syscall stack.
    "mov qword ptr [rip + sigrun_syscall_user_rsp], rsp",
    "mov rsp, qword ptr [rip + sigrun_syscall_kernel_rsp]",
    // Build SyscallFrame (push in reverse field order so [rsp] == rax).
    "push r11", // user RFLAGS  (field 14)
    "push rcx", // user RIP     (field 13)
    "push r15",
    "push r14",
    "push r13",
    "push r12",
    "push rbp",
    "push rbx",
    "push r10", // syscall arg3 (Linux ABI uses r10, not rcx)
    "push r9",  // syscall arg5
    "push r8",  // syscall arg4
    "push rdx", // syscall arg2
    "push rsi", // syscall arg1
    "push rdi", // syscall arg0
    "push rax", // syscall number  [rsp+0]
    // Pass frame pointer as first arg (System V: rdi).
    "mov rdi, rsp",
    "call rust_syscall_handler",
    // rax = return value from handler; discard the saved rax slot.
    "add rsp, 8",
    // Restore registers.
    "pop rdi",
    "pop rsi",
    "pop rdx",
    "pop r8",
    "pop r9",
    "pop r10",
    "pop rbx",
    "pop rbp",
    "pop r12",
    "pop r13",
    "pop r14",
    "pop r15",
    "pop rcx", // user RIP
    "pop r11", // user RFLAGS
    // Restore user RSP and return to user mode.
    "mov rsp, qword ptr [rip + sigrun_syscall_user_rsp]",
    "sysretq",
);

extern "C" {
    fn syscall_entry();
}

// ── MSR helpers ───────────────────────────────────────────────────────────────

unsafe fn rdmsr(reg: u32) -> u64 {
    let lo: u32;
    let hi: u32;
    core::arch::asm!(
        "rdmsr",
        in("ecx") reg,
        out("eax") lo,
        out("edx") hi,
        options(nomem, nostack, preserves_flags),
    );
    ((hi as u64) << 32) | lo as u64
}

unsafe fn wrmsr(reg: u32, value: u64) {
    core::arch::asm!(
        "wrmsr",
        in("ecx") reg,
        in("eax") value as u32,
        in("edx") (value >> 32) as u32,
        options(nomem, nostack, preserves_flags),
    );
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Enable SYSCALL/SYSRET and install the kernel entry stub.
pub fn init() {
    unsafe {
        // Point kernel RSP at our static syscall stack.
        let stack_top = SYSCALL_STACK_BUF.0.as_ptr().add(SYSCALL_STACK_SIZE) as u64;
        sigrun_syscall_kernel_rsp = stack_top;

        // Enable SYSCALL/SYSRET in EFER.SCE.
        wrmsr(MSR_EFER, rdmsr(MSR_EFER) | EFER_SCE);

        // STAR layout:
        //   [47:32] SYSCALL CS selector  → kernel code  (0x08)
        //           SYSCALL SS selector  = STAR[47:32]+8 = 0x10 (kernel data) ✓
        //   [63:48] SYSRETQ CS selector  = STAR[63:48]+16 = 0x20 → USER_CODE ✓
        //           SYSRETQ SS selector  = STAR[63:48]+8  = 0x18 → USER_DATA ✓
        // GDT must have user_data at 0x18 and user_code at 0x20 for this to work.
        let star: u64 = (0x0010_u64 << 48) | (0x0008_u64 << 32);
        wrmsr(MSR_STAR, star);

        // LSTAR = syscall_entry address.
        wrmsr(MSR_LSTAR, syscall_entry as u64);

        // SFMASK: clear IF (bit 9) on SYSCALL entry so we start with interrupts off.
        wrmsr(MSR_SFMASK, 0x200);

        crate::log::fmt(format_args!(
            "[SYSCALL] LSTAR={:#018x}  STAR={:#018x}",
            syscall_entry as u64, star
        ));
    }
}

/// Return the top of the syscall kernel stack (for TSS RSP0 configuration).
pub fn syscall_stack_top() -> u64 {
    unsafe { SYSCALL_STACK_BUF.0.as_ptr().add(SYSCALL_STACK_SIZE) as u64 }
}

// ── SyscallFrame ─────────────────────────────────────────────────────────────

/// Layout of the register save area pushed by `syscall_entry`.
///
/// `[rsp+0]` = `rax` (syscall number on entry; return value on exit).
#[repr(C)]
pub struct SyscallFrame {
    pub rax: u64, // syscall number / return value
    pub rdi: u64, // arg0
    pub rsi: u64, // arg1
    pub rdx: u64, // arg2
    pub r8: u64,  // arg4
    pub r9: u64,  // arg5
    pub r10: u64, // arg3 (syscall convention: r10 instead of rcx)
    pub rbx: u64,
    pub rbp: u64,
    pub r12: u64,
    pub r13: u64,
    pub r14: u64,
    pub r15: u64,
    pub rcx: u64, // saved user RIP
    pub r11: u64, // saved user RFLAGS
}
