# SIGRUN

A capability-based microkernel OS written in Rust, targeting x86_64.

```
SIGRUN Microkernel v0.1
=======================
Boot parameters validated
Initializing memory manager...     ✓
Setting up interrupt handling...   ✓
Initializing timer subsystem...    ✓
Initializing scheduler...          ✓
Initializing capability manager... ✓
Initializing IPC subsystem...      ✓
Starting scheduler...
SIGRUN kernel running
```

## Architecture

```
┌─────────────────────────────────────────┐
│              USERSPACE                   │
│   init  │  driver-mgr  │  shell  │  …   │
├─────────────────────────────────────────┤
│    IPC (capability-based messaging)      │
├─────────────────────────────────────────┤
│               KERNEL                     │
│  memory  │  scheduler  │  interrupts    │
│  timer   │  ipc        │  capabilities  │
├─────────────────────────────────────────┤
│    GRUB2 multiboot2 (BIOS / legacy)     │
└─────────────────────────────────────────┘
```

## Repository Layout

```
sigrun/
├── kernel/              # Microkernel core (x86_64-unknown-none)
│   ├── src/
│   │   ├── arch/        # x86_64: boot, GDT, IDT, APIC, serial
│   │   ├── memory/      # Frame allocator, page tables, mapper
│   │   ├── scheduler/   # Task table, round-robin scheduler
│   │   ├── timer/       # LAPIC timer (PIT-calibrated), HPET, wheel
│   │   ├── interrupt/   # IDT handlers, APIC routing
│   │   ├── ipc/         # Channels, endpoints, notifications, shared memory
│   │   └── capability/  # Capability tables and rights
│   ├── linker.ld        # Places kernel at 1 MB physical, GRUB-compatible layout
│   └── build.rs         # Passes linker script to rustc
├── userspace/
│   ├── init/            # PID 1 skeleton
│   ├── driver-manager/  # Driver framework
│   ├── filesystem/      # VFS server
│   ├── network/         # TCP/UDP stack
│   ├── shell/           # Command interpreter
│   └── common/          # Shared userspace utilities
├── libs/
│   ├── syscall-api/     # Syscall numbers and argument types
│   └── cap-std/         # Capability standard library
├── boot/                # Legacy boot stub (not active)
├── scripts/
│   ├── build.sh         # Build kernel + create bootable ISO
│   ├── run.sh           # Build + launch in QEMU
│   ├── test.sh          # Format check + clippy + unit tests
│   └── grub.cfg         # GRUB config template, copied into the ISO at build time
└── iso/                 # ISO staging area (generated, gitignored)
```

## Status

| Subsystem            | Status            |
|----------------------|-------------------|
| Boot (multiboot2)    | Working           |
| Serial output        | Working           |
| Memory (frames)      | Working           |
| Memory (paging)      | Working           |
| GDT / TSS            | Working           |
| IDT (all 256 vectors)| Working           |
| APIC / PIC           | Working           |
| LAPIC timer          | Working (PIT cal) |
| HPET                 | Simulated         |
| Timer wheel          | Working           |
| Scheduler            | Working (round-robin) |
| Capability system    | Scaffolded        |
| IPC channels         | Scaffolded        |
| IPC shared memory    | Scaffolded        |
| Userspace processes  | Scaffolded        |

## Prerequisites

### macOS

```bash
brew install rustup qemu i686-elf-grub mtools xorriso coreutils
rustup toolchain install nightly
rustup target add x86_64-unknown-none --toolchain nightly
```

### Linux (Debian/Ubuntu)

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup target add x86_64-unknown-none --toolchain nightly
apt install qemu-system-x86 grub-pc-bin xorriso mtools
```

## Quick Start

```bash
# Build kernel and create bootable ISO
./scripts/build.sh

# Run in QEMU (builds first)
./scripts/run.sh

# Boot test (exits with 0 on success)
./scripts/run.sh --test
```

## Build Options

```bash
./scripts/build.sh                # Release build (default)
./scripts/build.sh --debug        # Debug build (no optimisation, DWARF symbols)
./scripts/build.sh --check        # Type-check only (fast)

# Or with cargo directly (kernel only, no ISO):
cargo build -p kernel --target x86_64-unknown-none --release
```

## Run Options

```bash
./scripts/run.sh                  # Build and run (headless, serial console)
./scripts/run.sh --gui            # Open a real QEMU window
./scripts/run.sh --no-build       # Use existing sigrun.iso
./scripts/run.sh --memory=256M    # More RAM
./scripts/run.sh --cpus=2         # Multiple CPUs

# GDB debugging
./scripts/run.sh --gdb
# Then in another terminal:
gdb target/x86_64-unknown-none/debug/kernel -ex 'target remote :1234'
```

## Tests

```bash
./scripts/test.sh                 # Format check + clippy + unit tests
./scripts/test.sh --boot          # Also run QEMU boot test
./scripts/test.sh --all           # Same as --boot
./scripts/test.sh --no-clippy     # Skip clippy

# Individual checks:
cargo fmt --all -- --check
cargo clippy -p kernel --target x86_64-unknown-none
cargo test --workspace --exclude kernel
```

## Design Goals

- **Minimal kernel surface** — memory, scheduling, IPC, and capabilities only; everything else in userspace
- **Capability-based security** — all resource access goes through unforgeable capability references
- **Cloud/VM-aware** — virtio drivers, QEMU support, KVM acceleration on Linux
- **Correct over fast** — safe Rust with `unsafe` only where hardware demands it

## Contributing

1. Fork and create a branch
2. Run `cargo fmt --all` before committing
3. Run `./scripts/test.sh` to verify all checks pass
4. Open a pull request

See [ROADMAP.md](ROADMAP.md) for planned work.

## License

MIT OR Apache-2.0
