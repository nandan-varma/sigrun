# SIGRUN Roadmap

## Current State

The kernel compiles for `x86_64-unknown-none` and has a complete entry path:
bootloader → `kmain` → subsystem init (memory, interrupts, timer, scheduler, IPC,
capabilities) → COM1 serial output throughout.

Most subsystems are **scaffolded** — the data structures and module layout exist,
but the hardware interaction or algorithm is a stub that needs implementing.

---

## Phase 1 — Boot to Shell (near-term)

Goal: kernel boots in QEMU and drops into a minimal shell.

- [ ] **Bootloader** — finish ELF kernel loading and jump to `kmain` with a valid `BootParams`
- [ ] **Serial logging** — wire `log::*` to the HPET/UART clock for timestamped output (currently plain text only)
- [ ] **Memory** — initialize the buddy frame allocator from the UEFI memory map
- [ ] **Paging** — set up the higher-half kernel mapping and switch CR3
- [ ] **GDT / TSS** — load a proper GDT with a kernel stack in the TSS
- [ ] **IDT** — install all 256 exception and IRQ handlers
- [ ] **APIC** — initialize Local APIC, remap legacy PIC, enable timer interrupt
- [ ] **Scheduler** — basic round-robin: create idle task, switch context on timer tick
- [ ] **Syscall** — SYSCALL/SYSRET entry path, dispatch table
- [ ] **Init process** — spawn PID 1 from an in-memory ELF image
- [ ] **Shell** — minimal built-in commands (`ps`, `cap`, `exit`)

---

## Phase 2 — IPC and Capabilities

Goal: userspace processes communicate safely through the kernel.

- [ ] **IPC channels** — blocking `send`/`recv` with timeout
- [ ] **Shared memory** — zero-copy fast path between two processes
- [ ] **Notifications** — async bit-set notification objects
- [ ] **Capability tables** — per-process tables, `insert`/`get`/`remove`/`transfer`
- [ ] **Capability derivation** — mint restricted copies, track derivation depth
- [ ] **Revocation** — recursive revocation that propagates to derived caps
- [ ] **Driver manager** — service that owns device caps and hands them to drivers

---

## Phase 3 — Storage and Networking

Goal: persistent storage and network connectivity.

- [ ] **VirtIO-blk** — block device driver in userspace
- [ ] **VFS** — virtual filesystem layer in the filesystem server
- [ ] **FAT32 / ext2** — at least one on-disk format
- [ ] **Immutable root** — content-addressed, Merkle-validated read-only rootfs
- [ ] **VirtIO-net** — network device driver in userspace
- [ ] **IPv4 / TCP / UDP** — basic network stack
- [ ] **Socket API** — `bind`, `connect`, `send`, `recv` syscalls

---

## Phase 4 — Hardening and Performance

- [ ] **ARM64 port** — bring up on `aarch64-unknown-none`, share arch-agnostic code
- [ ] **Multi-core** — per-CPU runqueues, TLB shootdown, IPI handling
- [ ] **Miri / LOOM** — run unsafe kernel data structures under Miri; concurrent paths under Loom
- [ ] **Fuzzing** — fuzz syscall dispatch and IPC message paths
- [ ] **Formal verification** — explore applying SPARK-style invariants to the capability model
- [ ] **Benchmark suite** — IPC round-trip latency, context-switch overhead, memory throughput

---

## Known Gaps (won't compile end-to-end yet)

- `BootParams::validate()` always returns `true`; real memory map parsing is not wired up
- `scheduler::create_init_process()` and `scheduler::start()` are stubs that return immediately
- `interrupt::early_init()` / `timer::init()` / `ipc::init()` / `capability::init()` are stubs
- The kernel heap global allocator is a non-freeing bump allocator backed by a 4 MB static array;
  it must be replaced with the slab/buddy allocator before the kernel can handle real workloads
- Userspace processes run on the host (std) target for now and will need porting to `x86_64-unknown-none`
  with a proper syscall shim
