# SIGRUN - Next-Generation Capability-Based Microkernel OS

A production-grade, research-quality microkernel operating system written in Rust, designed for cloud-native and virtualization-aware workloads.

## Design Goals

- **Microkernel Architecture**: Minimal kernel with drivers, filesystems, and services in userspace
- **Capability-Based Security**: Object-capability model with unforgeable references
- **Immutable System**: Read-only root filesystem with atomic updates
- **Cloud-Native**: Optimized for virtualized environments (KVM, Xen, VMware)
- **Rust-First**: Memory-safe kernel with zero-CVV and formal verification paths

## Architecture

```
┌─────────────────────────────────────────┐
│           USERSPACE SERVICES             │
│  ┌────────┐ ┌────────┐ ┌────────────┐  │
│  │  Init  │ │ Driver │ │  Filesystem│  │
│  │Service │ │ Manager│ │   Server   │  │
│  └────┬───┘ └───┬────┘ └─────┬──────┘  │
└───────┼─────────┼────────────┼──────────┘
        │         │            │
        │    CAPABILITY IPC     │
        ▼         ▼            ▼
┌─────────────────────────────────────────┐
│            KERNEL CORE                  │
│  Scheduler │ IPC │ Memory │ Capability  │
│  Interrupt │ Timer                       │
└─────────────────────────────────────────┘
```

## Directory Structure

```
sigrun/
├── boot/           # UEFI bootloader
├── kernel/         # Microkernel core
├── userspace/      # User-space services
│   ├── init/      # PID 1 init service
│   ├── driver-manager/ # Driver framework
│   ├── filesystem/# Filesystem server
│   └── network/   # Network stack
├── libs/          # Shared libraries
└── build/         # Build scripts
```

## Quick Start

### Prerequisites

```bash
# Install Rust (nightly)
rustup install nightly
rustup target add x86_64-unknown-none

# Install QEMU with OVMF
brew install qemu  # macOS
# or
apt install qemu ovmf  # Linux
```

### Build

```bash
# Build everything
cargo build --workspace

# Build specific components
cargo build -p boot     # Bootloader
cargo build -p kernel   # Kernel
```

### Run in QEMU

```bash
./build/scripts/qemu.sh --kernel build/kernel.bin
```

## Development Tracks

The project is developed across 8 parallel tracks:

| Track | Agent | Area |
|-------|-------|------|
| 1 | Bootloader Engineer | UEFI boot, kernel entry |
| 2 | Memory Engineer | VMM, page tables |
| 3 | Scheduler Engineer | Task scheduling, timer |
| 4 | Interrupt Engineer | IDT, APIC |
| 5 | IPC Engineer | Message passing |
| 6 | Security Engineer | Capability system |
| 7 | Userspace Engineer | Init, services |
| 8 | Driver Engineer | Virtio, FS, network |

See [ROADMAP.md](ROADMAP.md) for detailed parallel development plan.

## Features

- [x] UEFI bootloader
- [ ] Virtual memory manager
- [ ] Multi-level feedback queue scheduler
- [ ] Lock-free IPC with shared memory fast path
- [ ] Object-capability security model
- [ ] Virtio drivers (block, network)
- [ ] Immutable root filesystem
- [ ] TCP/IP network stack
- [ ] Formal verification foundations

## Security Model

Every resource in SIGRUN is represented by a capability - an unforgeable reference that encodes access rights. This eliminates:
- No global namespaces
- No ambient authority
- No confused deputy problems

## Performance Targets

- Boot time: < 150ms (VM)
- IPC latency: < 2μs
- Kernel size: < 5MB
- Memory overhead: Minimal
- Scales to: 64 cores

## Testing

```bash
# Run all tests
cargo test --workspace

# Run QEMU integration tests
./build/scripts/test-qemu.sh

# Run with debug
./build/scripts/qemu.sh --debug --kernel build/kernel.bin
```

## License

MIT OR Apache-2.0

## References

- seL4: Formal verification microkernel
- Redox OS: Rust microkernel
- Fuchsia: Modern OS design
- Capability Hardware Extensions (CHERI)
