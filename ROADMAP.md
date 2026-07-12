# SIGRUN Roadmap

## Current State

The kernel boots end-to-end in QEMU via GRUB2 multiboot2. All core subsystems
initialise in sequence and the kernel enters an interrupt-driven idle loop.

Boot flow: `i686-elf-grub-mkrescue` ISO → GRUB2 → multiboot2 entry (`_start`, 32-bit)
→ 32→64 bit long-mode transition → `kmain_from_multiboot2` → `kmain`.

---

## Phase 1 — Boot to Idle Loop ✓ Complete

- [x] Multiboot2 header + 32-bit protected-mode entry
- [x] 32→64 bit long-mode transition (PAE, huge pages, EFER.LME)
- [x] Serial output (COM1, 115200 baud)
- [x] Memory: frame allocator from multiboot2 memory map
- [x] Memory: page table mapper
- [x] GDT with kernel code/data/TSS descriptors
- [x] TSS with kernel stack pointer
- [x] IDT: all 256 exception and IRQ vectors installed
- [x] Local APIC initialised; legacy PIC remapped and masked
- [x] LAPIC timer calibrated with PIT channel 2 (~10 ms periodic)
- [x] Timer wheel for soft timers
- [x] HPET (software-simulated for QEMU compatibility)
- [x] Round-robin task table; idle task
- [x] Capability table data structures
- [x] IPC channels, endpoints, notifications, shared memory (data structures + syscall stubs)
- [x] Global kernel heap (bump allocator, 4 MB BSS)

---

## Phase 2 — First Userspace Process

Goal: kernel spawns a PID 1 process that runs in ring 3.

- [ ] Context switch: save/restore general-purpose registers + stack pointer
- [ ] User-mode ELF loader: parse, map segments, set up stack and `argv`/`envp`
- [ ] SYSCALL/SYSRET entry path + syscall dispatch table
- [ ] `write` syscall wired to serial for init process output
- [ ] `exit` syscall removes task from scheduler
- [ ] Spawn PID 1 (`userspace/init`) from embedded ELF image
- [ ] Basic `ps`-style debug: list tasks over serial on a key press

---

## Phase 3 — IPC and Capabilities

Goal: userspace processes communicate safely through the kernel.

- [ ] IPC `send`/`recv` blocking paths with timeout
- [ ] Shared memory: zero-copy fast path between two processes
- [ ] Capability tables: per-process insert/get/remove/transfer
- [ ] Capability derivation: mint restricted copies, track depth
- [ ] Revocation: propagate recursively through derived capabilities
- [ ] Driver manager: owns device capabilities, hands them to drivers

---

## Phase 4 — Storage and Networking

Goal: persistent storage and network connectivity from userspace.

- [ ] VirtIO-blk driver in `userspace/driver-manager`
- [ ] VFS layer in `userspace/filesystem`
- [ ] FAT32 or ext2 on-disk format
- [ ] VirtIO-net driver
- [ ] TCP/UDP stack in `userspace/network`
- [ ] Socket syscalls: `bind`, `connect`, `send`, `recv`

---

## Phase 5 — Shell and Interactive Use

- [ ] Wire `userspace/shell` to the syscall layer
- [ ] Built-in commands: `ps`, `cap` (capability tree), `kill`, `echo`, `exit`
- [ ] Pipe between processes over IPC channels

---

## Phase 6 — Hardening and Performance

- [ ] Multi-core: per-CPU runqueues, TLB shootdown, IPI handling
- [ ] ARM64 port (`aarch64-unknown-none`), share arch-agnostic core
- [ ] Replace bump allocator with slab/buddy allocator
- [ ] Miri pass over unsafe kernel data structures
- [ ] Fuzz syscall dispatch and IPC message paths
- [ ] Benchmark suite: IPC round-trip latency, context-switch overhead

---

## Known Limitations

- Heap is a non-freeing bump allocator backed by a 4 MB static array
- `BootParams::validate()` always returns `true` (memory map not fully parsed)
- Userspace crates (`init`, `shell`, `network`, …) compile on the host target
  but are not yet spawned as real ring-3 processes
- No context switching or SYSCALL/SYSRET path yet
