# SIGRUN

A capability-based microkernel OS written in Rust, targeting x86_64 (ARM64 planned).

## Architecture

```
┌──────────────────────────────────────┐
│           USERSPACE                  │
│  init  │  driver-mgr  │  shell  │ … │
├──────────────────────────────────────┤
│   IPC (capability-based messaging)   │
├──────────────────────────────────────┤
│              KERNEL                  │
│  memory │ scheduler │ interrupt      │
│  timer  │ ipc       │ capability     │
├──────────────────────────────────────┤
│         UEFI BOOTLOADER              │
└──────────────────────────────────────┘
```

## Repository Layout

```
sigrun/
├── boot/              # UEFI bootloader
├── kernel/            # Microkernel core
│   └── src/
│       ├── arch/      # x86_64: GDT, IDT, APIC, serial
│       ├── memory/    # Frame allocator, page tables, heap
│       ├── scheduler/ # Tasks, runqueue, context switch
│       ├── timer/     # HPET, LAPIC timer, timer wheel
│       ├── interrupt/ # IDT handlers, APIC routing
│       ├── ipc/       # Channels, endpoints, shared memory
│       └── capability/# Capability tables and rights
├── userspace/
│   ├── init/          # PID 1
│   ├── driver-manager/# Driver framework + PCI/virtio
│   ├── filesystem/    # VFS server
│   ├── network/       # TCP/UDP stack
│   ├── shell/         # Command interpreter
│   └── common/        # Shared userspace utilities
├── libs/
│   ├── syscall-api/   # Syscall numbers and argument types
│   └── cap-std/       # Capability standard library
└── scripts/           # Build, run, and test helpers
```

## Status

| Subsystem       | Status         |
|-----------------|----------------|
| Bootloader      | In progress    |
| Memory (frame)  | Implemented    |
| Memory (paging) | Implemented    |
| Kernel heap     | Implemented    |
| Scheduler       | Scaffolded     |
| Timer (HPET)    | Scaffolded     |
| Interrupts      | Scaffolded     |
| IPC channels    | Scaffolded     |
| Capabilities    | Scaffolded     |
| Serial output   | Implemented    |
| Userspace init  | Scaffolded     |
| Network stack   | Scaffolded     |
| Filesystem      | Scaffolded     |

The kernel compiles and has a working entry point (`kmain`) that initialises subsystems
in order and writes boot messages over the COM1 serial port.

## Prerequisites

**Rust (nightly via rustup)**

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
rustup toolchain install nightly
rustup target add x86_64-unknown-none --toolchain nightly
rustup component add rust-src --toolchain nightly
```

**QEMU**

```bash
# macOS
brew install qemu

# Debian/Ubuntu
apt install qemu-system-x86
```

**OVMF** (UEFI firmware for QEMU)

```bash
# macOS — included with QEMU from Homebrew under:
# $(brew --prefix)/share/qemu/edk2-x86_64-code.fd

# Debian/Ubuntu
apt install ovmf
```

## Build

```bash
# Check everything compiles
cargo check --workspace

# Build bootloader
cargo build -p boot --target x86_64-unknown-none

# Build kernel
cargo build -p kernel --target x86_64-unknown-none

# Build all userspace
cargo build -p init -p driver-manager -p filesystem -p network -p shell

# Release build
cargo build --workspace --release
```

## Run in QEMU

```bash
./scripts/run.sh
```

Run with GDB debugging:

```bash
./scripts/run.sh --debug
# In another terminal:
gdb target/x86_64-unknown-none/debug/kernel -ex "target remote :1234"
```

## Test

```bash
# Unit tests (host target)
cargo test --workspace

# Clippy
cargo clippy --workspace -- -D warnings

# Format check
cargo fmt --all -- --check
```

## Design Goals

- **Minimal kernel surface** — memory, scheduling, IPC, and capabilities only; everything else in userspace
- **Capability-based security** — all resource access goes through unforgeable capability references
- **Immutable userspace** — read-only root filesystem with atomic updates (planned)
- **Cloud/VM-aware** — virtio drivers, OVMF/UEFI boot, KVM acceleration

## Contributing

1. Fork and create a branch (`git checkout -b feat/my-thing`)
2. Keep commits focused; run `cargo fmt --all` before committing
3. Add tests for new logic
4. Open a pull request — CI must pass

See [ROADMAP.md](ROADMAP.md) for planned work and known gaps.

## License

MIT OR Apache-2.0
