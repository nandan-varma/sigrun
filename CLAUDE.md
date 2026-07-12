# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

### Build

```bash
# Build kernel + produce sigrun.iso (release, default)
./scripts/build.sh

# Debug build (no optimisation, DWARF symbols retained)
./scripts/build.sh --debug

# Type-check only (fast, no linking or ISO)
./scripts/build.sh --check

# Kernel-only build without ISO
cargo build -p kernel --target x86_64-unknown-none --release
```

The `.cargo/config.toml` sets `target = "x86_64-unknown-none"` and compiles with `-C relocation-model=static -C code-model=kernel`. These flags are required and must not be removed — they allow the 32-bit boot code to use 32-bit absolute addresses; lld rejects R_X86_64_32 relocations without them.

`scripts/grub.cfg` is the canonical GRUB config; `scripts/build.sh` copies it into the ISO staging area at build time. Do not edit `iso/boot/grub/grub.cfg` directly.

### Run in QEMU

```bash
./scripts/run.sh              # build + boot (headless, serial to stdout)
./scripts/run.sh --no-build   # skip build, use existing sigrun.iso
./scripts/run.sh --gui        # open a QEMU window instead of headless
./scripts/run.sh --gdb        # pause QEMU; attach with gdb kernel -ex 'target remote :1234'
./scripts/run.sh --test       # boot test: exits 0 if "SIGRUN kernel running" appears
```

### Test / Lint

```bash
./scripts/test.sh            # rustfmt check + clippy + unit tests
./scripts/test.sh --boot     # also run QEMU boot test
./scripts/test.sh --all      # same

# Run a single test module
cargo test -p kernel -- ipc::tests      # kernel unit tests run on the host target
cargo test -p kernel -- timer::tests

# Clippy for kernel (must specify target)
cargo clippy -p kernel --target x86_64-unknown-none

# Format
cargo fmt --all
```

Unit tests inside the kernel crate compile on the host target (std) and are excluded from the cross-compiled binary. The `--target x86_64-unknown-none` flag is only needed for clippy/build, not for `cargo test`.

## Boot Flow

**Critical**: `qemu-system-x86_64 -kernel` does not work for 64-bit ELF in QEMU 5+. The only supported boot path is:

```
i686-elf-grub-mkrescue → sigrun.iso → qemu -cdrom sigrun.iso
```

GRUB2 BIOS (`i386-pc` target, from `i686-elf-grub` on macOS) loads the kernel via multiboot2. The `x86_64-elf-grub` Homebrew package only provides an EFI target and will silently fail to boot.

Boot sequence inside the kernel:
1. GRUB hands control to `_start` (32-bit protected mode, in `kernel/src/arch/x86_64/boot.rs`)
2. `_start` checks CPUID for long mode, sets up PML4+PDPT identity mapping (4×1GB huge pages), loads 64-bit GDT, enables PAE+EFER.LME+paging, far-jumps to `_boot_64_entry`
3. `_boot_64_entry` (64-bit) calls `kmain_from_multiboot2(magic, info_ptr)` in `kernel/src/main.rs`
4. `kmain_from_multiboot2` validates multiboot2 magic and calls `kmain`
5. `kmain` initialises subsystems in order and enters the idle loop via `scheduler::start()`

## Architecture

### Kernel (`kernel/`)

**`src/main.rs`** — binary crate root. Declares all modules, defines `kmain_from_multiboot2`, `kmain`, the global bump allocator, and the panic handler. **`src/lib.rs`** exists but is intentionally empty of `mod` declarations to avoid compiling `global_asm!`-defined `_start` twice (duplicate symbol at link time).

**`src/arch/`** — hardware abstraction.
- `mod.rs`: `BootParams`, `halt()`, `enable_interrupts()`, `PhysAddr`, `VirtAddr`
- `x86_64/boot.rs`: multiboot2 header + 32→64 transition in `global_asm!` (Intel syntax)
- `x86_64/gdt.rs`: GDT/TSS setup; `load_cs` uses a far return trick encoded as raw bytes
- `x86_64/idt.rs`: 256-entry IDT; `set_handler` / `set_handler_with_error` take `extern "x86-interrupt"` fn pointers
- `x86_64/apic.rs`: LocalAPIC (MMIO at 0xFEE00000), I/O APIC, legacy PIC; `outb`/`inb` use explicit `in("dx")`/`in("al")` register constraints
- `x86_64/serial.rs`: COM1 (0x3F8), same register constraints

**`src/interrupt/mod.rs`** — static `KERNEL_IDT` (must be `static`, not local, for `lidt` lifetime). All exception handlers are `extern "x86-interrupt"` inside `mod handlers`. Timer ISR (vector 32) calls `timer::on_tick()` then sends LAPIC EOI. `#![feature(abi_x86_interrupt)]` is declared in `main.rs`; `lib.rs` must not declare it.

**`src/memory/`** — `FrameAllocator` (buddy allocator), `AddressSpace` (owns a PML4 frame + region list), `MemoryManager`. The 4MB kernel heap is a non-freeing bump allocator (`static mut HEAP_MEMORY` in `main.rs`) — `static mut` is critical to place it in BSS, not `.rodata`.

**`src/timer/`** — `lapic.rs` calibrates the LAPIC timer using PIT channel 2 in one-shot mode (~10ms). `on_tick()` is called from the ISR, advances `CURRENT_TIME_NS` by 10ms, fires `TimerWheel`, and calls `scheduler::tick()`.

**`src/scheduler/`** — 64-slot `TaskTable` (flat array of `Option<Task>`) guarded by a `spin::Mutex`. Round-robin via `next_ready()`. **Never lock `TASK_TABLE` more than once in the same call stack** — `Mutex` is non-reentrant and will deadlock.

**`src/ipc/`** — channels, endpoints, notifications, shared memory, and syscall dispatch. All backed by `spin::Mutex`-protected managers held in an `OnceLock<IpcManager>` in `syscall.rs`. Has host-target unit tests.

**`src/capability/`** — `CapabilityTable` per process (fixed-size array of `Option<Capability>`), `CapabilityRegistry` (global map), rights bitmask, transfer/delegation.

**`src/log.rs`** — all output goes to COM1 serial. `log::fmt(format_args!(...))` for formatted output. `log::info_formatted` / `log::error_formatted` are aliases kept for backward compatibility.

### Linker script (`kernel/linker.ld`)

- Kernel loaded at physical `0x100000` (1 MB)
- Section order: `.multiboot2_header` → `.boot.text` → `.boot.data` → `.text` → `.rodata` → `.data` → `.boot.pgtbl` (NOLOAD) → `.boot.stack` (NOLOAD) → `.bss`
- `__kernel_end` symbol used by `kmain_from_multiboot2` to compute kernel size
- `.eh_frame*` and `.note.GNU-stack` discarded

### Userspace (`userspace/`)

Compile for the host (std) target. Not yet spawned as real ring-3 processes — they are compiled as standalone binaries for development/testing purposes only.

### Inline Assembly Rules

x86 port I/O instructions require explicit hardware registers — never use the generic `reg` class for byte values:
```rust
// Correct
asm!("out dx, al", in("dx") port, in("al") val, options(nomem, nostack, preserves_flags));
asm!("in al, dx",  in("dx") port, out("al") val, options(nomem, nostack, preserves_flags));

// Correct for 16-bit segment registers
asm!("mov ss, ax", in("ax") selector, options(nomem, nostack, preserves_flags));
asm!("ltr ax",     in("ax") selector, options(nomem, nostack, preserves_flags));
```

Numeric-only local labels (`0:`, `1:`) are rejected by LLVM — use labels starting with a letter or a digit ≥ 2.
