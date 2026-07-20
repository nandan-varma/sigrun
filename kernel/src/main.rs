//! SIGRUN Microkernel — main entry point.

#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]
#![allow(unsafe_op_in_unsafe_fn)]
#![cfg_attr(target_arch = "x86_64", feature(abi_x86_interrupt))]

extern crate alloc;

mod arch;
mod capability;
mod error;
mod interrupt;
mod ipc;
mod log;
mod memory;
mod scheduler;
mod syscall;
mod timer;

use arch::BootParams;

// ── Global bump allocator (4 MB, lives in BSS) ───────────────────────────────

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{AtomicUsize, Ordering};

const HEAP_SIZE: usize = 4 * 1024 * 1024;

#[repr(align(4096))]
struct StaticHeap([u8; HEAP_SIZE]);

static mut HEAP_MEMORY: StaticHeap = StaticHeap([0; HEAP_SIZE]);
static HEAP_NEXT: AtomicUsize = AtomicUsize::new(0);

struct BumpAllocator;

unsafe impl GlobalAlloc for BumpAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let base = HEAP_MEMORY.0.as_ptr() as usize;
        loop {
            let cur = HEAP_NEXT.load(Ordering::Relaxed);
            let aligned = (cur + layout.align() - 1) & !(layout.align() - 1);
            let next = aligned + layout.size();
            if next > HEAP_SIZE {
                return core::ptr::null_mut();
            }
            if HEAP_NEXT
                .compare_exchange_weak(cur, next, Ordering::SeqCst, Ordering::Relaxed)
                .is_ok()
            {
                return (base + aligned) as *mut u8;
            }
        }
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

unsafe impl Sync for BumpAllocator {}

#[cfg_attr(not(test), global_allocator)]
static GLOBAL_ALLOC: BumpAllocator = BumpAllocator;

// ── Boot entry (from multiboot2 assembly) ────────────────────────────────────

#[unsafe(no_mangle)]
pub unsafe extern "C" fn kmain_from_multiboot2(magic: u32, info_ptr: u64) -> ! {
    use arch::x86_64::boot::MB2_BOOTLOADER_MAGIC;
    log::early_init();

    if magic != MB2_BOOTLOADER_MAGIC {
        log::error("Bad multiboot2 magic");
        halt();
    }

    extern "C" {
        static __kernel_end: u8;
    }
    let kernel_end = core::ptr::addr_of!(__kernel_end) as u64;

    let boot_params = BootParams {
        magic: 0x5349_4752,
        version: 1,
        memory_map: info_ptr as *mut u8,
        memory_map_size: 0,
        memory_descriptor_size: 0,
        kernel_phys_start: 0x10_0000,
        kernel_virt_start: 0x10_0000,
        kernel_size: kernel_end.saturating_sub(0x10_0000),
        rsdp_address: 0,
        efi_system_table: 0,
    };

    kmain(&boot_params);
}

// ── Kernel main ───────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn kmain(boot_params: &BootParams) -> ! {
    log::early_init();

    log::info("╔══════════════════════════════════════════════╗");
    log::info("║   SIGRUN Microkernel  v0.2.0  (x86_64)      ║");
    log::info("╚══════════════════════════════════════════════╝");
    log::info("");

    if !boot_params.validate() {
        log::error("Invalid boot parameters!");
        halt();
    }
    log::fmt(format_args!(
        "[BOOT ] Kernel: phys={:#x}  size={} KB",
        boot_params.kernel_phys_start,
        boot_params.kernel_size / 1024
    ));

    // ── Memory ────────────────────────────────────────────────────────────────
    log::info("[MEM  ] Initializing buddy frame allocator...");
    let _memory = memory::init(boot_params);
    log::fmt(format_args!(
        "[MEM  ] Heap: {} MB static bump allocator",
        HEAP_SIZE / (1024 * 1024)
    ));

    // ── Enable user-mode page access ──────────────────────────────────────────
    unsafe { enable_user_pages() };
    log::info("[MEM  ] First 4 GB marked user-accessible (U/S bit set)");

    // ── Interrupts / GDT / IDT / APIC ────────────────────────────────────────
    log::info("[INT  ] Setting up GDT, IDT, APIC...");
    interrupt::early_init();
    log::fmt(format_args!(
        "[INT  ] TSS.RSP0={:#x}  IST1={:#x}",
        arch::x86_64::gdt::ist_stack_top(),
        arch::x86_64::gdt::ist_stack_top()
    ));

    // ── Timer ─────────────────────────────────────────────────────────────────
    log::info("[TMR  ] Calibrating LAPIC timer...");
    timer::init();
    log::info("[TMR  ] LAPIC timer: 100 Hz (10 ms period)");

    // ── Subsystems ────────────────────────────────────────────────────────────
    log::info("[SCHED] Initializing preemptive scheduler...");
    let _sched = scheduler::init();

    log::info("[CAP  ] Initializing capability manager...");
    let _caps = capability::init();

    log::info("[IPC  ] Initializing IPC subsystem...");
    ipc::init();
    log::info("[IPC  ] Channels / notifications / shared-memory ready");

    // ── SYSCALL/SYSRET ────────────────────────────────────────────────────────
    log::info("[SYSCALL] Enabling SYSCALL/SYSRET (LSTAR)...");
    arch::x86_64::syscall::init();

    // ── Kernel tasks ──────────────────────────────────────────────────────────
    log::info("");
    log::info("[TASK ] Spawning kernel tasks:");
    scheduler::spawn_kernel_task("heartbeat", heartbeat_task, scheduler::Priority::new(64));
    scheduler::spawn_kernel_task("counter", counter_task, scheduler::Priority::new(128));
    scheduler::spawn_kernel_task("ipc-demo", ipc_demo_task, scheduler::Priority::new(128));
    scheduler::register_idle_task();
    log::info("[TASK ] Task table ready");

    // ── Ring-3 userspace demo ─────────────────────────────────────────────────
    log::info("");
    log::info("[RING3] Spawning init process (PID 1) in ring 3...");
    unsafe { launch_user_demo() };
    // Returns here after sys_exit is called by user code.
    log::info("[RING3] Init process returned to kernel");

    // ── Start preemptive scheduler ────────────────────────────────────────────
    log::info("");
    log::fmt(format_args!(
        "[SCHED] Entering idle loop — {} kernel tasks ready",
        3 // heartbeat + counter + ipc-demo
    ));
    scheduler::start()
}

// ── Enable user-mode pages ───────────────────────────────────────────────────

unsafe fn enable_user_pages() {
    extern "C" {
        static _boot_pml4: u8;
        static _boot_pdpt: u8;
    }
    let pml4 = core::ptr::addr_of!(_boot_pml4) as *mut u64;
    let pdpt = core::ptr::addr_of!(_boot_pdpt) as *mut u64;

    // Set U/S (bit 2) on PML4[0] and PDPT[0..3] so ring-3 can access all
    // identity-mapped memory.  (Real kernel would restrict this per-process.)
    *pml4 |= 0x4;
    for i in 0..4usize {
        *pdpt.add(i) |= 0x4;
    }

    // Flush TLB by reloading CR3.
    core::arch::asm!(
        "mov {tmp}, cr3",
        "mov cr3, {tmp}",
        tmp = out(reg) _,
        options(nostack, preserves_flags),
    );
}

// ── Ring-3 userspace demo ─────────────────────────────────────────────────────

/// The "init process" that runs at privilege level 3.
///
/// It calls sys_write to print to serial, queries its PID via sys_getpid,
/// and terminates with sys_exit.  Because all first-4GB pages have U/S=1,
/// this Rust function can execute from ring 3 even though it's linked into
/// the kernel binary.
#[no_mangle]
pub extern "C" fn user_init_process() -> ! {
    const MSG: &[u8] = b"[USER ] Hello from ring 3! SYSCALL sys_write works.\n";
    unsafe {
        // sys_write(fd=1, buf, len)
        core::arch::asm!(
            "syscall",
            inout("rax") 1u64 => _,
            in("rdi") 1u64,
            in("rsi") MSG.as_ptr() as u64,
            in("rdx") MSG.len() as u64,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        // sys_getpid()
        let pid: u64;
        core::arch::asm!(
            "syscall",
            inout("rax") 39u64 => pid,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        // Print pid via another sys_write (format string lives in .rodata, also accessible)
        let _ = pid; // We'll just log that we got it
        const MSG2: &[u8] = b"[USER ] sys_getpid() returned successfully\n";
        core::arch::asm!(
            "syscall",
            inout("rax") 1u64 => _,
            in("rdi") 1u64,
            in("rsi") MSG2.as_ptr() as u64,
            in("rdx") MSG2.len() as u64,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
        // sys_exit(0)
        core::arch::asm!(
            "syscall",
            in("rax") 60u64,
            in("rdi") 0u64,
            out("rcx") _,
            out("r11") _,
            options(nostack),
        );
    }
    loop {} // unreachable after sys_exit
}

/// Kernel-side: allocate a user stack and IRETQ to `user_init_process`.
///
/// When the user process calls sys_exit, `switch_to_idle()` is called which
/// performs a context switch away and never returns here.
unsafe fn launch_user_demo() {
    // Allocate user stack (4 KB from kernel heap — accessible by ring-3
    // because U/S is set for the whole first 4 GB).
    use alloc::alloc::{alloc, Layout};
    let user_stack_layout = Layout::from_size_align(4096, 16).unwrap();
    let user_stack_bottom = alloc(user_stack_layout);
    assert!(!user_stack_bottom.is_null(), "user stack alloc failed");
    let user_stack_top = (user_stack_bottom as usize + 4096) as u64;

    let user_rip = user_init_process as u64;

    log::fmt(format_args!(
        "[RING3] IRETQ → RIP={:#x}  CS=0x23  RSP={:#x}  RFLAGS=0x202",
        user_rip, user_stack_top
    ));

    // IRETQ frame: SS, RSP, RFLAGS, CS, RIP  (pushed high-to-low)
    core::arch::asm!(
        "push {ss}",
        "push {user_rsp}",
        "pushfq",
        "or qword ptr [rsp], 0x200",  // ensure IF=1 in pushed rflags
        "push {cs}",
        "push {user_rip}",
        "iretq",
        ss      = in(reg) 0x1bu64,   // USER_DATA | 3
        user_rsp= in(reg) user_stack_top,
        cs      = in(reg) 0x23u64,   // USER_CODE | 3
        user_rip= in(reg) user_rip,
        options(noreturn),
    );
}

// ── Kernel task functions ─────────────────────────────────────────────────────

/// Heartbeat: logs a timestamp every ~1 second.
fn heartbeat_task() -> ! {
    let mut beat: u64 = 0;
    loop {
        beat += 1;
        let ms = timer::current_time() / 1_000_000;
        log::fmt(format_args!(
            "[BEAT ] #{:>4}  T={} ms  ticks={}",
            beat,
            ms,
            scheduler::tick_count()
        ));

        // Spin ~1 s worth of timer ticks (100 ticks × 10 ms = 1 000 ms).
        // The preemptive scheduler will context-switch us out during this wait.
        let deadline = timer::current_time() + 1_000_000_000;
        while timer::current_time() < deadline {
            core::hint::spin_loop();
        }
    }
}

/// Counter: tight loop demonstrating that the scheduler preempts CPU-bound work.
fn counter_task() -> ! {
    let mut n: u64 = 0;
    loop {
        n = n.wrapping_add(1);
        if n % 50_000_000 == 0 {
            log::fmt(format_args!(
                "[CTR  ] {} M iterations  T={} ms",
                n / 1_000_000,
                timer::current_time() / 1_000_000
            ));
        }
    }
}

/// IPC demo: bidirectional channels, notifications, and capability tables.
fn ipc_demo_task() -> ! {
    use ipc::{
        channel::Channel,
        endpoint::ProcessId,
        message::{Message, MessageType},
        notification::NotificationBits,
        syscall::get_manager,
    };

    log::info("[IPC  ] IPC demo task starting");

    let pid_a = ProcessId::new();
    let pid_b = ProcessId::new();

    // ── Channel messaging ──────────────────────────────────────────────────────
    let channel = Channel::create(pid_a, pid_b).expect("channel create");
    log::fmt(format_args!(
        "[IPC  ] Channel created: ep_a={:?} ep_b={:?}",
        channel.endpoint_a.endpoint.id, channel.endpoint_b.endpoint.id
    ));

    for seq in 1u64..=3 {
        let msg = Message::new(MessageType::Send, pid_a.as_u64());
        channel.send_from_a(msg).expect("send_from_a");
        log::fmt(format_args!("[IPC  ] A→B message #{} sent", seq));

        let recv = channel.recv_at_b().expect("recv_at_b");
        log::fmt(format_args!(
            "[IPC  ] B received #{}: sender_pid={}",
            seq, recv.header.sender_pid
        ));
    }

    // ── Notifications (async bit signals) ──────────────────────────────────────
    let manager = get_manager();
    let notif = manager.notifications.create_notification(pid_a);
    manager
        .notifications
        .signal(
            notif.id,
            NotificationBits::BIT_0.bits() | NotificationBits::BIT_1.bits(),
        )
        .ok();
    let bits = notif.wait(NotificationBits::BIT_0.bits());
    log::fmt(format_args!(
        "[IPC  ] Notification bits received: {:#010b}",
        bits
    ));

    // ── Capability table ───────────────────────────────────────────────────────
    let _cap_table = capability::CapabilityTable::new(pid_a.as_u64());
    log::fmt(format_args!(
        "[CAP  ] Per-process capability table: {} slots",
        capability::table::MAX_CAPABILITIES
    ));

    log::info("[IPC  ] IPC + capability demo complete");
    loop {
        crate::arch::halt();
    }
}

// ── Panic handler & halt ──────────────────────────────────────────────────────

fn halt() -> ! {
    loop {
        arch::halt();
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    log::fmt(format_args!("KERNEL PANIC: {}", info));
    halt()
}
