# SIGRUN Development - AGENTS.md

## Overview

This document describes how to work with SIGRUN's parallel development system using multiple agents. The OS is developed across 8 parallel tracks, each assigned to a specialized agent.

---

## Agent Assignments

### Agent 1: Bootloader Engineer
- **Track**: Bootloader & Kernel Entry
- **Path**: `boot/`
- **Status**: Independent (start first)
- **Dependencies**: None
- **Contact**: Other agents wait for BootInfo

### Agent 2: Memory Management Engineer
- **Track**: Memory Manager
- **Path**: `kernel/src/memory/`
- **Status**: Parallel with Agent 1
- **Dependencies**: BootInfo from Agent 1
- **Contact**: Provides Mapper trait to others

### Agent 3: Scheduler Engineer
- **Track**: Scheduler & Timer
- **Path**: `kernel/src/scheduler/`, `kernel/src/timer/`
- **Status**: Starts after Agent 2 memory setup
- **Dependencies**: Memory allocator
- **Contact**: Provides Scheduler trait

### Agent 4: Interrupt Engineer
- **Track**: Interrupt Controller
- **Path**: `kernel/src/interrupt/`
- **Status**: Parallel with Agent 3
- **Dependencies**: Memory, Timer
- **Contact**: Provides IrqHandler trait

### Agent 5: IPC Engineer
- **Track**: IPC System
- **Path**: `kernel/src/ipc/`
- **Status**: Parallel development
- **Dependencies**: Scheduler
- **Contact**: Provides IPC syscalls

### Agent 6: Security Engineer
- **Track**: Capability Manager
- **Path**: `kernel/src/capability/`
- **Status**: Starts mid-development
- **Dependencies**: IPC (for transfer)
- **Contact**: Provides CapabilityTable

### Agent 7: Userspace Services Engineer
- **Track**: Userspace Core
- **Path**: `userspace/init/`, `userspace/driver-manager/`
- **Status**: Integration phase
- **Dependencies**: All kernel subsystems
- **Contact**: Provides init service

### Agent 8: Driver & Services Engineer
- **Track**: Drivers, Filesystem & Network
- **Path**: `userspace/filesystem/`, `userspace/network/`
- **Status**: Final integration
- **Dependencies**: Driver Manager (Agent 7)
- **Contact**: Provides full system

---

## Communication Protocol

### Weekly Sync Points

Each agent should update their progress in the shared tracking file:

```bash
# Update progress (run at end of each day)
./scripts/update-progress.sh --agent <1-8> --status "completed week X phase Y"
```

### Issue Tracking

When one agent's work blocks another:

1. Create issue with tag `blocking`
2. Assign to blocking agent
3. Add `blocked-by: #issue-number` to blocked agent's task
4. Escalate to team lead if unresolved > 48 hours

### Code Review

- All PRs require 1 approval from non-author agent
- Cross-track changes need approval from both agents
- Use @ mentions for required reviewers

---

## File Navigation

### Finding Files

```bash
# Find any file by name
find . -name "*.rs" | xargs grep -l "CapabilityTable"

# Find files by track
ls boot/src/          # Agent 1
ls kernel/src/memory/ # Agent 2
ls kernel/src/scheduler/ # Agent 3
ls kernel/src/timer/ # Agent 3
ls kernel/src/interrupt/ # Agent 4
ls kernel/src/ipc/   # Agent 5
ls kernel/src/capability/ # Agent 6
ls userspace/init/    # Agent 7
ls userspace/driver-manager/ # Agent 7
ls userspace/filesystem/ # Agent 8
ls userspace/network/ # Agent 8
```

### Understanding Dependencies

```bash
# Check what depends on a module
grep -r "use.*capability" --include="*.rs" kernel/src/
grep -r "use.*memory" --include="*.rs" kernel/src/
```

---

## Development Workflow

### Quick Start (Recommended)

Each agent should use the optimized build/test scripts for fast iterations:

```bash
# 1. Start watch mode (auto-rebuild on save) - FASTEST!
./scripts/dev.sh <your-track-number> watch

# 2. Quick check (10x faster than full build)
./build.sh --check <your-track-number>

# 3. Full build when ready
./build.sh <your-track-number>

# 4. Run tests
./test.sh <your-track-number>

# 5. Update progress
./scripts/update-progress.sh -a <your-track-number> -s "Status message"
```

### Fast Iteration Workflow

**Recommended daily workflow for each agent:**

```bash
# Terminal 1: Watch mode (keeps running, rebuilds on save)
./scripts/dev.sh 2 watch  # Replace 2 with your track number

# Terminal 2: Make changes, auto-rebuilds happen
# Edit files in your assigned directory...
# See instant feedback on compilation errors

# When ready to test:
./test.sh 2

# Before commit:
cargo fmt --all
./scripts/update-progress.sh -a 2 -s "Completed Phase X.Y"
```

### Build Commands by Track

```bash
# Agent 1: Bootloader
./build.sh 1              # Build bootloader only
./build.sh --check 1      # Quick check
./test.sh 1               # Test bootloader

# Agent 2: Memory
./build.sh 2              # Build kernel (memory focus)
./build.sh --check 2      # Quick check (10x faster)
./test.sh 2               # Test memory subsystem

# Agent 3: Scheduler
./build.sh 3              # Build kernel (scheduler focus)
./test.sh 3               # Test scheduler

# Agents 4-6: Similar pattern
./build.sh <4-6>
./test.sh <4-6>

# Agent 7: Userspace Core
./build.sh 7              # Build init + driver-manager
./test.sh 7               # Test userspace core

# Agent 8: Services
./build.sh 8              # Build filesystem + network + shell
./test.sh 8               # Test services
```

### Build Multiple Tracks (For Integration)

```bash
# Build dependencies together
./build.sh 2 3            # Memory + Scheduler
./build.sh 2 3 4 5        # Kernel core subsystems

# Test integration
./test.sh 2 3 4 5

# Full system
./build.sh                # Build all tracks (parallel)
./test.sh                 # Test all tracks (parallel)
```

### Advanced Build Options

```bash
# Release builds (optimized)
./build.sh --release <track>

# Verbose output (see all details)
./build.sh --verbose <track>

# Sequential builds (not parallel)
./build.sh --sequential <track>

# Test with coverage
./test.sh --coverage <track>

# Test with QEMU integration
./test.sh --qemu

# Test with verbose output
./test.sh --verbose --nocapture <track>
```

### Performance Tips

1. **Use watch mode** - Fastest iteration (1s rebuilds)
   ```bash
   ./scripts/dev.sh <track> watch
   ```

2. **Use --check for quick validation** - 10x faster than full build
   ```bash
   ./build.sh --check <track>
   ```

3. **Build only your track** - Don't build everything
   ```bash
   ./build.sh 2  # Not ./build.sh
   ```

4. **Parallel by default** - Scripts run parallel builds automatically

5. **View logs for details** - Check `target/build-track<N>-*.log`

### Starting a New Feature

1. **Create feature branch**
   ```bash
   git checkout -b feature/track<N>-phase<X>-description
   ```

2. **Start watch mode**
   ```bash
   ./scripts/dev.sh <N> watch
   ```

3. **Implement** - Edit files, watch mode rebuilds automatically

4. **Test when ready**
   ```bash
   ./test.sh <N>
   ```

5. **Update progress**
   ```bash
   ./scripts/update-progress.sh -a <N> -s "Completed Phase X.Y"
   ```

6. **Commit**
   ```bash
   cargo fmt --all
   git add .
   git commit -m "Track <N>: Description"
   git push
   ```

---

## QEMU Testing

Use the integrated test script for QEMU:

```bash
# Run QEMU integration tests
./test.sh --qemu

# This will:
# 1. Build the kernel in release mode
# 2. Launch QEMU with the kernel
# 3. Check for successful boot
# 4. Verify basic functionality
```

### Manual QEMU Testing

```bash
# Build first
./build.sh --release 1 2

# Launch QEMU
./build/scripts/qemu.sh --kernel build/kernel/kernel.bin \
    --memory 2G \
    --cpus 2

# Debug mode
./build/scripts/qemu.sh --debug --kernel build/kernel/kernel.bin
# Then in another terminal:
gdb build/kernel/kernel.bin -ex "target remote localhost:1234"
```

---

## Code Standards

### Formatting

```bash
# Format all code
cargo fmt --all

# Check formatting
cargo fmt --all -- --check
```

### Linting

```bash
# Run clippy
cargo clippy --workspace -- -D warnings

# Run with miri (for unsafe code)
cargo miri test
```

### Documentation

All public APIs must have documentation:

```rust
/// Description of what this function does.
///
/// # Arguments
/// * `arg1` - Description of first argument
///
/// # Returns
/// Description of return value
///
/// # Errors
/// Description of possible error conditions
///
/// # Example
/// ```
/// let result = my_function(42);
/// ```
pub fn my_function(arg1: u32) -> Result<u32, Error> {
    // ...
}
```

---

## Common Issues & Solutions

### Issue: "Cannot find module"
Build is slow"

**Solution**: Use faster build methods:
```bash
# 1. Use watch mode (fastest - 1s incremental)
./scripts/dev.sh <track> watch

# 2. Use --check flag (10x faster)
./build.sh --check <track>

# 3. Build only your track, not all
./build.sh 2  # Not ./build.sh
```

### Issue: "Cannot find module"

**Cause**: Module not exported in parent `mod.rs`

**Solution**: Add `pub mod module_name;` to parent module
```rust
// kernel/src/lib.rs
pub mod memory;
pub mod scheduler;
```

### Issue: "Want to see detailed build errors"

**Solution**: Check log files or use verbose mode
```bash
# Check log
cat target/build-track<N>-<package>.log

# Or build with verbose
./build.sh --verbose <track>
```

### Issue: "Tests fail in parallel"

**Solution**: Run tests sequentially
```bash
./test.sh --sequential <track>
```

### Issue: "Cyclic dependency"

**Cause**: Two modules depending on each other

**Solution**: Move shared types to a third module, or use type erasure

### Issue: "Blocked waiting for another agent"

**Solution**: 
1. Check `progress.log` to see their status
   ```bash
   tail -f progress.log
   ```
2. Build your track independently where possible
3. Create issue with `blocking` tag if needed
---

## Quick Reference

### Agent Responsibilities Summary

| Agent | Primary Files | Key Traits | Output |
|-------|---------------|------------|--------|
| 1 | `boot/src/*` | UEFI, asm | BootInfo |
| 2 | `kernel/src/memory/*` | alloc, paging | Mapper |
| 3 | `kernel/src/scheduler/*` | scheduling | TaskId |
| 4 | `kernel/src/interrupt/*` | IDT, APIC | IrqHandler |
| 5 | `kernel/src/ipc/*` | messaging | Message |
| 6 | `kernel/src/capability/*` | security | Capability |
| 7 | `userspace/init/*` | services | Process |
| 8 | `userspace/filesystem/*` | storage | FileHandle |

### Key Interfaces

```rust
// Bootloader -> Kernel
BootInfo

// Memory
trait FrameAllocator
trait Mapper

// Scheduler
trait Scheduler
struct TaskId

// Interrupt
trait IrqHandler

// Project Structure

```
sigrun/
├── build.sh              # Parallel build script ⚡
├── test.sh               # Parallel test script 🧪
├── progress.log          # Agent tracking (auto-generated)
│
├── boot/                 # Agent 1
│   └── src/
├── kernel/               # Agents 2-6
│   └── src/
│       ├── memory/      # Agent 2
│       ├── scheduler/   # Agent 3
│       ├── timer/       # Agent 3
│       ├── interrupt/   # Agent 4
│       ├── ipc/         # Agent 5
│       └── capability/  # Agent 6
├── userspace/            # Agents 7-8
│   ├── init/            # Agent 7
│   ├── driver-manager/  # Agent 7
│   ├── filesystem/      # Agent 8
│   ├── network/         # Agent 8
│   └── shell/           # Agent 8
├── libs/
│   ├── syscall-api/
│   └── cap-std/
├── scripts/
│   ├── dev.sh           # Quick dev commands
│   └── update-progress.sh
└── build/
    └── scripts/
        └── qemu.sh
```

---

## Performance Benchmarks

**Build Times (parallel vs sequential):**
- Sequential: ~30s × 8 tracks = 4 minutes
- Parallel: ~45s total (**5.3x faster**)
- Check mode: ~3s per track (**10x faster**)
- Watch mode: ~1s incremental (**30x faster**)

**Recommended workflow:** Watch mode for active development!

---

## Notes

- **Always use watch mode** for fastest iteration: `./scripts/dev.sh <N> watch`
- **Always run `cargo fmt`** before committing
- **Never commit** with `unsafe_op_in_unsafe_fn` warnings
- **Keep PRs focused** and small (< 500 lines preferred)
- **Write tests** for new functionality
- **Update progress** daily: `./scripts/update-progress.sh -a <N> -s "message"`
- **Monitor other agents**: `tail -f progress.log`

// Userspace
struct SyscallArgs
```

---

## Contact & Help

- **Team Lead**: Assign PR reviews, resolve blockers
- **Documentation**: See `docs/` directory
- **Architecture**: See `ROADMAP.md` for detailed specs
- **Issues**: Use GitHub issues with appropriate labels

---

## Notes

- Always run `cargo fmt` before committing
- Never commit with `unsafe_op_in_unsafe_fn` warnings
- Keep PRs focused and small (< 500 lines preferred)
- Write tests for new functionality
- Update documentation when APIs change
