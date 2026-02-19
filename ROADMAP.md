# SIGRUN Operating System - Parallel Development Roadmap

## Project Overview

SIGRUN is a production-grade, research-quality microkernel OS written in Rust with:
- Capability-based security model (like seL4)
- Immutable system architecture
- Cloud-native and virtualization-aware design
- Target: x86_64 + ARM64

This roadmap defines 8 parallel development tracks that can be executed simultaneously with minimal dependencies.

---

# TRACK 1: Bootloader & Kernel Entry

## Agent 1: Bootloader Engineer

### Overview
Responsible for UEFI bootloader, kernel loading, and early kernel initialization. This track is completely independent and should be the first to start.

### Responsibilities
1. UEFI application development
2. Memory map parsing
3. Kernel loading and relocation
4. Boot parameter creation
5. Early paging setup (identity mapping)

### Deliverables

#### Phase 1.1: UEFI Bootloader Foundation (Week 1-2)
- [ ] Create boot/Cargo.toml workspace member
- [ ] Set up no_std UEFI application
- [ ] Implement UEFI system table bindings
- [ ] Create console output for early debugging
- [ ] Basic disk I/O for reading kernel

#### Phase 1.2: Memory Management (Week 3)
- [ ] Parse UEFI memory map (EFI_MEMORY_DESCRIPTOR)
- [ ] Implement memory type classification
- [ ] Create boot memory allocator (bump allocator)
- [ ] Handle memory reservations

#### Phase 1.3: Kernel Loading (Week 4)
- [ ] ELF kernel loading support
- [ ] Kernel image relocation
- [ ] Parse kernel ELF sections
- [ ] Create BootInfo structure:
  ```rust
  struct BootInfo {
      memory_map: MemoryMap,
      kernel_phys_start: PhysAddr,
      kernel_virt_start: VirtAddr,
      kernel_size: usize,
      rsdp_address: PhysAddr,
      efi_system_table: *mut (),
  }
  ```

#### Phase 1.4: Early Paging Setup (Week 5)
- [ ] Create identity-mapped page tables
- [ ] Setup temporary 1:1 mapping for early kernel
- [ ] Enable long mode on x86_64
- [ ] Jump to kernel entry point

### File Structure Created
```
boot/
├── Cargo.toml
├── src/
│   ├── main.rs              # UEFI entry point
│   ├── efi/
│   │   ├── mod.rs
│   │   ├── console.rs       # Console I/O
│   │   ├── memory.rs       # Memory map parsing
│   │   └── fs.rs           # File system access
│   ├── kernel.rs           # Kernel loading
│   ├── paging.rs           # Boot-time paging
│   └── params.rs           # BootInfo structure
└── uefi/
    └── uefi-sys/           # Raw UEFI bindings
```

### Dependencies
- **Input**: None (greenfield)
- **Output**: BootInfo structure for kernel

### Testing Strategy
- Build UEFI application with cargo
- Test with OVMF (Open Virtual Machine Firmware)
- QEMU: `qemu-system-x86_64 -bios OVMF.fd`

### Key Interfaces (ABI)
```rust
// Passed from bootloader to kernel
#[repr(C)]
pub struct BootInfo {
    pub magic: u64,              // 0x53494752 ("SIGR")
    pub version: u32,
    pub memory_map: PhysAddr,
    pub memory_map_size: usize,
    pub memory_descriptor_size: usize,
    pub kernel_phys_start: PhysAddr,
    pub kernel_virt_start: PhysAddr,
    pub kernel_size: usize,
    pub rsdp_address: PhysAddr,
    pub efi_system_table: PhysAddr,
}
```

### Reference Code Location
- `kernel/src/arch/x86_64/boot.s` - Assembly entry point
- `kernel/src/main.rs` - Kernel entry receives BootInfo

---

# TRACK 2: Memory Management System

## Agent 2: Memory Management Engineer

### Overview
Implements the virtual memory manager (VMM), page table management, and frame allocation. This track is independent and critical for all other tracks.

### Responsibilities
1. Frame allocator (buddy system)
2. Page table management (4-level on x86_64)
3. Virtual address space management
4. Memory mapping operations
5. Page fault handling interface

### Deliverables

#### Phase 2.1: Frame Allocator (Week 1-2)
- [ ] Implement buddy allocator for physical frames
- [ ] Support multiple page sizes (4KB, 2MB, 1GB)
- [ ] Initialize from memory map
- [ ] Add frame tracking and statistics

```rust
// kernel/src/memory/frame.rs
pub struct FrameAllocator {
    buddy: BuddyAllocator,
    total_frames: usize,
    used_frames: AtomicUsize,
}

impl FrameAllocator {
    pub fn allocate(&self, order: usize) -> Option<PhysFrame>;
    pub fn allocate_sized(&self, size: usize) -> Option<PhysFrame>;
    pub fn deallocate(&self, frame: PhysFrame, order: usize);
    pub fn stats(&self) -> AllocatorStats;
}
```

#### Phase 2.2: Page Table Structure (Week 3)
- [ ] Define page table entry types for x86_64
- [ ] Implement PML4, PDPT, PD, PT structures
- [ ] Create page table entry flags

```rust
// kernel/src/memory/page_table.rs
#[repr(align(4096))]
pub struct Pml4([Pml4Entry; 512]);

pub struct Pml4Entry(u64);
impl Pml4Entry {
    pub fn new(frame: PhysFrame, flags: PageTableFlags) -> Self;
    pub fn is_present(&self) -> bool;
    pub fn frame(&self) -> PhysFrame;
    pub fn flags(&self) -> PageTableFlags;
}

#[derive(Clone, Copy)]
pub struct PageTableFlags {
    pub present: bool,
    pub writable: bool,
    pub user_accessible: bool,
    pub write_through: bool,
    pub no_cache: bool,
    pub accessed: bool,
    pub dirty: bool,
    pub no_execute: bool,
    pub global: bool,
}
```

#### Phase 2.3: Virtual Memory Manager (Week 4-5)
- [ ] Implement AddressSpace type
- [ ] Create mapping functions (map/unmap/protect)
- [ ] Implement address space switching
- [ ] Add page fault handler interface

```rust
// kernel/src/memory/mod.rs
pub struct AddressSpace {
    pub id: AddressSpaceId,
    pml4: PhysFrame,
    regions: BTreeMap<VirtRange, MemoryRegion>,
    refcount: Arc<AtomicU32>,
}

pub trait Mapper {
    fn map(&mut self, virt: VirtAddr, phys: PhysFrame, flags: PageTableFlags) -> Result<()>;
    fn unmap(&mut self, virt: VirtAddr) -> Result<PhysFrame>;
    fn update_flags(&mut self, virt: VirtAddr, flags: PageTableFlags) -> Result<()>;
    fn query(&self, virt: VirtAddr) -> Result<PageQuery>;
}

pub struct PageQuery {
    pub present: bool,
    pub frame: Option<PhysFrame>,
    pub flags: PageTableFlags,
}
```

#### Phase 2.4: Kernel Address Space (Week 6)
- [ ] Create kernel AS with permanent mappings
- [ ] Set up vmalloc region
- [ ] Implement early allocator for kernel use
- [ ] Create vm_allocate interface

### File Structure Created
```
kernel/src/memory/
├── mod.rs                 # Main exports
├── addr.rs                # Address types (VirtAddr, PhysAddr)
├── frame.rs               # Frame allocator
├── page_table.rs         # Page table structures
├── mapper.rs              # Mapping operations
├── region.rs              # Memory regions
├── heap.rs                # Kernel heap (bump + slab)
└── error.rs               # Memory error types
```

### Dependencies
- **Input**: BootInfo from Track 1 (memory map)
- **Output**: Mapper trait for all kernel components

### Key Interfaces
```rust
// Frame types
pub struct PhysFrame { start: PhysAddr, order: usize }
pub struct VirtAddr(u64);
pub struct PhysAddr(u64);

// Mapper interface (used by other subsystems)
pub fn create_address_space() -> Result<AddressSpace>;
pub fn switch_to_space(space: &AddressSpace);
pub fn map_pages(space: &mut AddressSpace, virt: VirtAddr, phys: PhysFrame, flags: Flags) -> Result<()>;
```

---

# TRACK 3: Scheduler & Timer Subsystem

## Agent 3: Scheduler Engineer

### Overview
Implements the multi-core scheduler with priority support and the timer subsystem for timekeeping and scheduling ticks.

### Responsibilities
1. Task/thread representation
2. Multi-level priority runqueue
3. Context switching
4. Timer subsystem (HPET, LAPIC)
5. Sleeping and wakeup mechanisms

### Deliverables

#### Phase 3.1: Task Representation (Week 1-2)
- [ ] Define TaskId, TaskState, Task structures
- [ ] Implement task context (registers, stack pointer)
- [ ] Create task storage (slot-based)
- [ ] Add task lifecycle management

```rust
// kernel/src/scheduler/task.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Ready,
    Running,
    Blocked(BlockReason),
    Sleeping(Deadline),
    Terminated,
}

pub struct Task {
    pub id: TaskId,
    pub state: TaskState,
    pub priority: Priority,
    pub cpu: CpuId,
    pub affinity: CpuMask,
    pub time_slice: Nanoseconds<u64>,
    pub kernel_stack: VirtAddr,
    pub user_stack: VirtAddr,
    pub context: TaskContext,
    pub address_space: AddressSpaceId,
    pub capabilities: CapabilityTableHandle,
    pub wakeup_time: Option<Deadline>,
}

pub struct TaskContext {
    pub rip: u64,
    pub rsp: u64,
    pub rflags: u64,
    // ... callee-saved registers
}
```

#### Phase 3.2: Runqueue Implementation (Week 3)
- [ ] Implement per-CPU runqueues
- [ ] Multi-level feedback queue (MLFQ)
- [ ] Priority inheritance support
- [ ] Load balancing between CPUs

```rust
// kernel/src/scheduler/runqueue.rs
pub struct Runqueue {
    pub cpu: CpuId,
    pub priority_queues: [Vec<TaskId>; PRIORITY_LEVELS],
    pub current: Option<TaskId>,
    pub idle_task: TaskId,
    pub lock: Spinlock<RunqueueInner>,
}

impl Runqueue {
    pub fn enqueue(&self, task: TaskId, priority: Priority);
    pub fn dequeue(&self) -> Option<TaskId>;
    pub fn yield_current(&self) -> Option<TaskId>;
    pub fn wake(&self, task: TaskId);
}
```

#### Phase 3.3: Context Switching (Week 4)
- [ ] Implement task switch assembly code
- [ ] Save/restore user context
- [ ] Handle kernel/user transitions
- [ ] Add stack switching

```asm
# kernel/asm/x86_64/task_switch.s
.section .text
.global task_switch
.type task_switch, @function

task_switch:
    # Save current task state
    pushfq
    push %rbp
    push %rbx
    push %r12
    push %r13
    push %r14
    push %r15
    
    # Store RSP to current task
    mov %rsp, [rdi + Task.context + 0]
    
    # Load new task state
    mov [rsi + Task.context + 0], %rsp
    
    pop %r15
    pop %r14
    pop %r13
    pop %r12
    pop %rbx
    pop %rbp
    popfq
    
    ret
```

#### Phase 3.4: Timer Subsystem (Week 5-6)
- [ ] HPET driver for x86_64
- [ ] Local APIC timer integration
- [ ] Clock source abstraction
- [ ] Timer wheel for scheduled tasks

```rust
// kernel/src/timer/mod.rs
pub trait ClockSource: Send + Sync {
    fn now(&self) -> Timestamp;
    fn resolution(&self) -> Duration;
    fn name(&self) -> &'static str;
}

pub struct Timestamp {
    pub nanoseconds: u64,
}

pub struct TimerWheel {
    wheels: [Vec<TimerEntry>; TIMER_WHEEL_LEVELS],
    current_tick: u64,
}

impl TimerWheel {
    pub fn schedule(&mut self, deadline: Deadline, callback: Box<dyn TimerCallback>) -> TimerId;
    pub fn cancel(&mut self, id: TimerId);
    pub fn tick(&mut self) -> Vec<TimerId>;
}
```

### File Structure Created
```
kernel/src/scheduler/
├── mod.rs
├── task.rs
├── runqueue.rs
├── priority.rs
├── affinity.rs
├── context.rs              # Assembly context switch
└── idle.rs                # Idle task

kernel/src/timer/
├── mod.rs
├── clock.rs               # Timekeeping
├── hpet.rs                # HPET driver
├── lapic.rs               # LAPIC timer
├── timeout.rs             # Timeout management
└── wheel.rs               # Timer wheel
```

### Dependencies
- **Input**: Memory allocator from Track 2
- **Output**: Scheduler trait, timer interfaces

### Key Interfaces
```rust
pub trait Scheduler: Send + Sync {
    fn schedule(&self) -> Option<TaskRef>;
    fn enqueue(&self, task: TaskRef);
    fn wake(&self, task: TaskRef);
    fn sleep_until(&self, deadline: Deadline) -> Result<(), SleepError>;
    fn yield_now(&self);
    fn set_priority(&self, task: TaskRef, priority: Priority) -> Result<()>;
}
```

---

# TRACK 4: Interrupt Controller

## Agent 4: Interrupt Engineer

### Overview
Implements interrupt handling infrastructure including IDT, APIC, and interrupt routing. Critical for multi-core and device support.

### Responsibilities
1. Interrupt Descriptor Table (IDT) setup
2. APIC initialization and management
3. Interrupt handler registration
4. IRQ routing and affinity
5. Exception handling

### Deliverables

#### Phase 4.1: IDT and Exception Handling (Week 1-2)
- [ ] Set up IDT with all exception handlers
- [ ] Implement double fault handler
- [ ] Page fault handler (calls VMM)
- [ ] System call handler (SYSCALL/SYSENTER)

```rust
// kernel/src/interrupt/idt.rs
pub struct Idt {
    entries: [IdtEntry; 256],
}

#[repr(C)]
pub struct IdtEntry {
    pub offset_low: u16,
    pub selector: u16,
    pub ist: u8,
    pub type_attr: u8,
    pub offset_mid: u16,
    pub offset_high: u32,
    pub reserved: u32,
}

pub struct InterruptFrame {
    pub rip: u64,
    pub cs: u16,
    pub rflags: u64,
    pub rsp: u64,
    pub ss: u16,
    // Error code for some interrupts
    pub error_code: Option<u64>,
}
```

#### Phase 4.2: APIC Infrastructure (Week 3-4)
- [ ] Initialize Local APIC
- [ ] Set up I/O APIC
- [ ] Implement interrupt routing
- [ ] Add MSI support

```rust
// kernel/src/interrupt/apic.rs
pub struct LocalApic {
    base: PhysAddr,
    mmio: &'static mut [u32],
}

impl LocalApic {
    pub fn init(&mut self);
    pub fn enable(&self);
    pub fn set_priority(&self, vector: u8);
    pub fn eoi(&self);
    pub fn id(&self) -> u32;
    pub fn start_timer(&self, vector: u8, mode: TimerMode);
}

pub struct IoApic {
    mmio: &'static mut [u32],
    irq_count: u8,
}

impl IoApic {
    pub fn redirect(&mut self, irq: u8, vector: u8, dest: ApicId, polarity: Polarity);
}
```

#### Phase 4.3: Interrupt Handler Framework (Week 5)
- [ ] Create handler registration API
- [ ] Implement per-IRQ handlers
- [ ] Add interrupt threading support
- [ ] Handle spurious interrupts

```rust
// kernel/src/interrupt/mod.rs
pub trait IrqHandler: Send {
    fn handle(&self, frame: &mut InterruptFrame) -> IrqResult;
    fn irq(&self) -> IrqNumber;
}

pub fn register_handler(irq: IrqNumber, handler: Box<dyn IrqHandler>) -> Result<(), IrqError>;
pub fn enable_irq(irq: IrqNumber);
pub fn disable_irq(irq: IrqNumber);
pub fn set_affinity(irq: IrqNumber, cpu: CpuMask);
```

#### Phase 4.4: Exception Handling (Week 6)
- [ ] Divide error handler
- [ ] Invalid opcode handler
- [ ] GPF handler
- [ ] Machine check handler (MCE)

### File Structure Created
```
kernel/src/interrupt/
├── mod.rs
├── idt.rs                 # IDT setup
├── handler.rs             # Handler framework
├── apic.rs                # APIC management
├── pic.rs                 # Legacy PIC
├── msi.rs                 # MSI support
├── vector.rs              # Vector management
└── exception.rs           # Exception types
```

### Dependencies
- **Input**: Memory allocator (Track 2), Timer (Track 3)
- **Output**: IrqHandler trait for drivers

---

# TRACK 5: IPC System

## Agent 5: IPC Engineer

### Overview
Implements the inter-process communication subsystem - critical for the microkernel architecture. This includes message passing, shared memory, and async notifications.

### Responsibilities
1. IPC channel creation and management
2. Message format and serialization
3. Fast path (shared memory)
4. Endpoint management
5. Async notifications

### Deliverables

#### Phase 5.1: Message Types (Week 1-2)
- [ ] Define message structures
- [ ] Implement capability transfer in messages
- [ ] Create message header format
- [ ] Add inline payload support

```rust
// kernel/src/ipc/message.rs
pub const MAX_INLINE_CAPS: usize = 4;
pub const MAX_INLINE_PAYLOAD: usize = 256;

pub struct Message {
    pub header: MessageHeader,
    pub inline_caps: [Option<CapabilityId>; MAX_INLINE_CAPS],
    pub inline_payload: [u8; MAX_INLINE_PAYLOAD],
    pub payload_len: usize,
    pub extra_caps_handle: u32,
}

#[repr(C)]
pub struct MessageHeader {
    pub size: u32,
    pub cap_count: u8,
    pub msg_type: MessageType,
    pub sender_pid: u64,
    pub priority: u8,
    pub flags: MessageFlags,
}

#[derive(Debug, Clone, Copy)]
pub enum MessageType {
    Call,      // Request-response
    Send,      // Fire-and-forget
    Recv,      // Blocking receive
    Signal,    // Async notification
    ShareMemory, // Shared memory region
}
```

#### Phase 5.2: IPC Channels (Week 3-4)
- [ ] Create IpcChannel type
- [ ] Implement message queue
- [ ] Add endpoint management
- [ ] Create channel creation API

```rust
// kernel/src/ipc/channel.rs
pub struct IpcChannel {
    pub local: Endpoint,
    pub remote: Endpoint,
    pub rights: IpcRights,
    pub queue: Arc<MessageQueue>,
}

pub struct Endpoint {
    pub process: ProcessId,
    pub slot: CapabilitySlot,
}

pub struct MessageQueue {
    ring: RingBuffer<Message>,
    producer_idx: AtomicU64,
    consumer_idx: AtomicU64,
}

pub struct IpcRights {
    pub send: bool,
    pub recv: bool,
    pub grant: bool,  // Transfer capabilities
}

impl IpcChannel {
    pub fn create() -> Result<(Endpoint, Endpoint), IpcError>;
    pub fn send(&self, msg: Message, timeout: Option<Deadline>) -> Result<Message, IpcError>;
    pub fn recv(&self, timeout: Option<Deadline>) -> Result<Message, IpcError>;
}
```

#### Phase 5.3: Fast Path - Shared Memory (Week 5)
- [ ] Implement shared memory regions
- [ ] Add memory region transfer in IPC
- [ ] Create doorbell mechanism for notification
- [ ] Optimize for zero-copy

```rust
// kernel/src/ipc/shmem.rs
pub struct SharedMemoryRegion {
    pub page_frames: Vec<PhysFrame>,
    pub page_count: usize,
    pub rights: MemoryRights,
    pub share_mode: ShareMode,
}

pub enum ShareMode {
    Copy,      // Copy on write
    ReadOnly,  // Read-only mapping
    ReadWrite, // Writable mapping
}

impl IpcChannel {
    pub fn share_memory(&self, region: SharedMemoryRegion) -> Result<ShmHandle, IpcError>;
    pub fn receive_shared_memory(&self, handle: ShmHandle) -> Result<SharedMemoryRegion, IpcError>;
}
```

#### Phase 5.4: Async Notifications (Week 6)
- [ ] Implement notification objects
- [ ] Add select/wait operations
- [ ] Create event multiplexing

```rust
// kernel/src/ipc/notification.rs
pub struct Notification {
    pub process: ProcessId,
    pub slots: [AtomicU64; 4],  // 4 * 64 = 256 bits
}

impl Notification {
    pub fn signal(&self, bits: u64);
    pub fn wait(&self, mask: u64, deadline: Option<Deadline>) -> Result<u64, WaitError>;
    pub fn bind_to_channel(&self, channel: &IpcChannel, bit: u8);
}
```

### File Structure Created
```
kernel/src/ipc/
├── mod.rs
├── message.rs             # Message types
├── channel.rs             # IPC channels
├── queue.rs               # Message queue (lock-free)
├── endpoint.rs            # Endpoints
├── shared_memory.rs       # Fast path shmem
├── notification.rs        # Async notifications
└── syscall.rs            # Syscall interface
```

### Dependencies
- **Input**: Scheduler (Track 3), Capability Manager (Track 6 - can mock)
- **Output**: IPC syscall numbers for userspace

---

# TRACK 6: Capability Manager

## Agent 6: Security Engineer

### Overview
Implements the capability-based security model - the heart of the OS security architecture. This track defines how all resources are accessed.

### Responsibilities
1. Capability table management
2. Object reference handling
3. Rights management and derivation
4. Capability revocation
5. Cross-process delegation

### Deliverables

#### Phase 6.1: Core Capability Types (Week 1-2)
- [ ] Define CapabilityId and Capability types
- [ ] Implement CapabilityRights bitflags
- [ ] Create ObjectType enum
- [ ] Add capability derivation

```rust
// kernel/src/capability/mod.rs
use bitflags::bitflags;

bitflags! {
    pub struct CapabilityRights: u32 {
        const NONE = 0;
        const READ = 1 << 0;
        const WRITE = 1 << 1;
        const EXECUTE = 1 << 2;
        const DELETE = 1 << 3;
        const ADMIN = 1 << 4;
        const GRANT = 1 << 5;
        const MAP = 1 << 6;
        const SIGNAL = 1 << 7;
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct CapabilityId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectType {
    Process,
    Thread,
    AddressSpace,
    Endpoint,
    Frame,
    Device,
    Irq,
    IoPort,
    Timer,
    Notification,
}

pub struct Capability {
    pub id: CapabilityId,
    pub object_type: ObjectType,
    pub object_id: u64,
    pub rights: CapabilityRights,
    pub derivation_depth: u8,
    pub flags: CapabilityFlags,
}

pub bitflags! {
    pub struct CapabilityFlags: u8 {
        const NONE = 0;
        const SYSTEM = 1 << 0;     // System-wide capability
        const REVOKABLE = 1 << 1; // Can be revoked
        const PERMANENT = 1 << 2;  // Cannot be revoked
    }
}
```

#### Phase 6.2: Capability Tables (Week 3-4)
- [ ] Create per-process capability tables
- [ ] Implement slot allocation
- [ ] Add capability lookup
- [ ] Create insert/remove operations

```rust
// kernel/src/capability/table.rs
pub struct CapabilityTable {
    pub process_id: ProcessId,
    slots: Vec<Option<CapabilityEntry>>,
    next_free: SlotAllocator,
    lock: RwLock<()>,
}

pub struct CapabilityEntry {
    pub capability: Capability,
    pub derivation_chain: Vec<CapabilityId>,
}

impl CapabilityTable {
    pub fn new(process_id: ProcessId, initial_slots: usize) -> Self;
    pub fn insert(&self, cap: Capability) -> Result<CapabilitySlot, CapError>;
    pub fn get(&self, slot: CapabilitySlot) -> Option<&Capability>;
    pub fn remove(&self, slot: CapabilitySlot) -> Option<Capability>;
    pub fn transfer(&self, slot: CapabilitySlot, target: &CapabilityTable) 
        -> Result<CapabilitySlot, CapError>;
}
```

#### Phase 6.3: Derivation and Rights (Week 5)
- [ ] Implement capability derivation (mint)
- [ ] Add rights reduction
- [ ] Create derivation chains
- [ ] Implement copy vs move semantics

```rust
// kernel/src/capability/derivation.rs
impl Capability {
    pub fn derive(&self, new_rights: CapabilityRights) -> Option<Capability> {
        let derived_rights = self.rights.intersection(new_rights);
        
        if derived_rights.is_empty() {
            return None;
        }
        
        if self.derivation_depth >= MAX_DERIVATION_DEPTH {
            return None;
        }
        
        Some(Capability {
            id: CapabilityId::new(),
            object_type: self.object_type,
            object_id: self.object_id,
            rights: derived_rights,
            derivation_depth: self.derivation_depth + 1,
            flags: self.flags & CapabilityFlags::PERMANENT,
        })
    }
    
    pub fn can(&self, required: CapabilityRights) -> bool {
        self.rights.contains(required)
    }
}
```

#### Phase 6.4: Revocation (Week 6)
- [ ] Implement recursive revocation
- [ ] Create revocation propagation
- [ ] Add lazy vs immediate revocation
- [ ] Handle revocation in IPC

```rust
// kernel/src/capability/revocation.rs
pub struct RevocationResult {
    pub revoked_count: usize,
    pub errors: Vec<(CapabilityId, CapError)>,
}

pub trait RevocationManager {
    fn revoke(&self, cap: CapabilityId) -> Result<RevocationResult, CapError>;
    fn revoke_recursive(&self, cap: CapabilityId) -> Result<RevocationResult, CapError>;
    fn check_valid(&self, cap: CapabilityId) -> bool;
}
```

### File Structure Created
```
kernel/src/capability/
├── mod.rs                 # Main exports
├── cap.rs                 # Capability types
├── table.rs               # Per-process tables
├── derivation.rs          # Derivation operations
├── revocation.rs          # Revocation logic
├── rights.rs              # Rights definitions
├── namespace.rs           # Capability namespaces
├── error.rs               # Error types
└── syscall.rs            # Capability syscalls
```

### Dependencies
- **Input**: IPC (Track 5) for capability transfer
- **Output**: Capability trait for all kernel objects

---

# TRACK 7: Userspace Core Services

## Agent 7: Userspace Services Engineer

### Overview
Implements the initial user-space services including init, driver manager, and basic shell. This track brings the system to a usable state.

### Responsibilities
1. Init service (PID 1)
2. Driver manager
3. Basic system services
4. Capability distribution
5. Service manifest parsing

### Deliverables

#### Phase 7.1: System Call Interface (Week 1-2)
- [ ] Define syscall numbers
- [ ] Create syscall handler in kernel
- [ ] Implement userspace syscall wrapper
- [ ] Add basic syscalls (exit, fork, exec, read, write)

```rust
// libs/syscall-api/src/number.rs
pub const SYSCALL_EXIT: u64 = 0;
pub const SYSCALL_FORK: u64 = 1;
pub const SYSCALL_EXEC: u64 = 2;
pub const SYSCALL_READ: u64 = 3;
pub const SYSCALL_WRITE: u64 = 4;
pub const SYSCALL_OPEN: u64 = 5;
pub const SYSCALL_CLOSE: u64 = 6;
pub const SYSCALL_MMAP: u64 = 7;
pub const SYSCALL_MUNMAP: u64 = 8;
pub const SYSCALL_IPC: u64 = 9;
pub const SYSCALL_CAP: u64 = 10;
pub const SYSCALL_PROCESS: u64 = 11;
pub const SYSCALL_THREAD: u64 = 12;
pub const SYSCALL_SCHED: u64 = 13;
pub const SYSCALL_TIME: u64 = 14;

// libs/syscall-api/src/arg.rs
#[repr(C)]
pub struct SyscallArgs {
    pub num: u64,
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64,
    pub arg3: u64,
    pub arg4: u64,
    pub arg5: u64,
}

pub unsafe fn syscall(args: SyscallArgs) -> Result<u64, SyscallError>;
```

#### Phase 7.2: Init Service (Week 3-4)
- [ ] Create init process (PID 1)
- [ ] Implement service manifest parsing
- [ ] Add process spawning
- [ ] Create capability distribution

```rust
// userspace/init/src/main.rs
#![no_std]
#![forbid(unsafe_op_in_unsafe_fn)]

extern crate syscall_api;

pub fn main() -> ! {
    // Get initial capabilities from kernel
    let init_caps = get_initial_capabilities();
    
    // Create service manager
    let mut sm = ServiceManager::new(init_caps);
    
    // Parse and load service manifest
    let manifest = sm.load_manifest("/etc/services.toml")
        .expect("Failed to load manifest");
    
    // Start all services
    sm.start_services(&manifest).expect("Failed to start services");
    
    // Wait for child services and handle events
    loop {
        handle_service_events();
    }
}
```

#### Phase 7.3: Driver Manager (Week 5-6)
- [ ] Implement PCI enumeration
- [ ] Create driver framework
- [ ] Add virtio driver detection
- [ ] Implement driver lifecycle

```rust
// userspace/driver-manager/src/main.rs
pub struct DriverManager {
    pub pci: PciBus,
    pub drivers: DriverRegistry,
    pub device_tree: DeviceTree,
}

impl DriverManager {
    pub fn init() -> Result<Self>;
    pub fn enumerate_pci(&mut self);
    pub fn load_driver(&mut self, device: &PciDevice) -> Result<DriverHandle>;
    pub fn handle_interrupt(&mut self, irq: IrqNumber);
    pub fn get_device(&self, id: DeviceId) -> Option<&dyn Driver>;
}
```

#### Phase 7.4: Basic Shell (Week 7)
- [ ] Simple command interpreter
- [ ] Basic file operations
- [ ] Process listing
- [ ] Capability inspection

```rust
// userspace/shell/src/main.rs
pub struct Shell {
    pub current_caps: CapabilitySet,
    pub prompt: String,
}

impl Shell {
    pub fn run(&mut self) -> !;
    pub fn parse_command(&self, line: &str) -> Command;
    pub fn execute(&mut self, cmd: Command) -> Result<Output>;
}
```

### File Structure Created
```
libs/syscall-api/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── number.rs          # Syscall numbers
│   ├── arg.rs             # Argument types
│   ├── error.rs           # Error codes
│   └── syscall.rs         # Syscall wrapper

userspace/
├── Cargo.toml
├── common/
│   └── src/
│       ├── lib.rs
│       ├── error.rs
│       └── ipc.rs
├── init/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── service.rs
│       └── manifest.rs
├── driver-manager/
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── pci.rs
│       ├── driver.rs
│       └── virtio.rs
└── shell/
    ├── Cargo.toml
    └── src/
        ├── main.rs
        └── interpreter.rs
```

### Dependencies
- **Input**: All kernel subsystems (1-6)
- **Output**: Complete userspace system

---

# TRACK 8: Drivers, Filesystem & Network

## Agent 8: Driver & Services Engineer

### Overview
Implements the virtio drivers, filesystem server, and network stack. This track provides the system with storage and networking capabilities.

### Responsibilities
1. Virtio block and network drivers
2. Filesystem server (VFS)
3. Network stack
4. Immutable root filesystem
5. COW/snapshot support

### Deliverables

#### Phase 8.1: Virtio Drivers (Week 1-3)
- [ ] Implement virtio ring
- [ ] Create virtio-blk driver
- [ ] Add virtio-net driver
- [ ] Implement virtio-gpu (optional)

```rust
// userspace/driver-manager/src/virtio/mod.rs
pub struct VirtioDevice {
    pub device_type: VirtioDeviceType,
    pub features: VirtioFeatures,
    pub queue: [VirtQueue; 4],
    pub config: VirtioConfig,
}

pub enum VirtioDeviceType {
    Block = 1,
    Network = 2,
    Console = 3,
    Gpu = 16,
}

pub struct VirtQueue {
    pub desc: PhysAddr,
    pub avail: PhysAddr,
    pub used: PhysAddr,
    pub size: u16,
}

impl VirtioDevice {
    pub fn probe(pci: &PciDevice) -> Option<Self>;
    pub fn negotiate_features(&mut self, supported: u64) -> u64;
    pub fn setup_queues(&mut self, queues: &[(usize, usize)]);
    pub fn notify(&self, queue_idx: u16);
}

// userspace/driver-manager/drivers/virtio-blk/src/lib.rs
pub struct VirtioBlkDriver {
    device: VirtioDevice,
    capacity: u64,
    block_size: u32,
}

impl BlockDevice for VirtioBlkDriver {
    fn read(&mut self, sector: u64, buffer: &mut [u8]) -> Result<(), IoError>;
    fn write(&mut self, sector: u64, data: &[u8]) -> Result<(), IoError>;
    fn flush(&mut self) -> Result<(), IoError>;
}
```

#### Phase 8.2: VFS Implementation (Week 4-5)
- [ ] Create virtual file system layer
- [ ] Implement in-memory file descriptors
- [ ] Add path resolution
- [ ] Create mount points

```rust
// userspace/filesystem/src/vfs.rs
pub struct Vfs {
    pub root: Arc<VfsNode>,
    pub mounts: BTreeMap<String, Arc<dyn FileSystem>>,
}

pub trait FileSystem: Send + Sync {
    fn mount(&self, path: &str) -> Result<(), FsError>;
    fn open(&self, path: &str, flags: OpenFlags) -> Result<FileHandle, FsError>;
    fn read(&self, file: FileHandle, buf: &mut [u8]) -> Result<usize, FsError>;
    fn write(&self, file: FileHandle, data: &[u8]) -> Result<usize, FsError>;
    fn stat(&self, path: &str) -> Result<FileStat, FsError>;
}
```

#### Phase 8.3: Immutable Root Filesystem (Week 6-7)
- [ ] Create read-only root fs
- [ ] Implement content-addressable store
- [ ] Add Merkle tree validation
- [ ] Implement COW layer

```rust
// userspace/filesystem/src/immutable.rs
pub struct ImmutableFs {
    pub cas: ContentAddressableStore,
    pub snapshots: BTreeMap<String, SnapshotId>,
    pub cow_layer: CopyOnWriteLayer,
}

pub struct ContentAddressableStore {
    pub store: MappedFile,
    pub index: BTreeMap<ContentHash, PhysRegion>,
}

impl ContentAddressableStore {
    pub fn put(&mut self, data: &[u8]) -> Result<ContentHash, FsError>;
    pub fn get(&self, hash: &ContentHash) -> Option<&[u8]>;
    pub fn verify(&self, hash: &ContentHash, data: &[u8]) -> bool;
}
```

#### Phase 8.4: Network Stack (Week 8-10)
- [ ] Ethernet driver interface
- [ ] IPv4 implementation
- [ ] TCP/UDP stack
- [ ] Basic socket API

```rust
// userspace/network/src/stack.rs
pub struct NetworkStack {
    pub eth: EthernetLayer,
    pub ipv4: Ipv4Layer,
    pub tcp: TcpLayer,
    pub udp: UdpLayer,
    pub sockets: SocketTable,
}

impl NetworkStack {
    pub fn recv_packet(&mut self, frame: &[u8]);
    pub fn send_packet(&mut self, packet: &mut PacketBuffer) -> Result<(), NetError>;
    pub fn bind_socket(&mut self, proto: Protocol, port: Port) -> Result<SocketId, NetError>;
}
```

### File Structure Created
```
userspace/driver-manager/drivers/
├── virtio-blk/
│   ├── Cargo.toml
│   └── src/lib.rs
├── virtio-net/
│   ├── Cargo.toml
│   └── src/lib.rs
└── virtio-gpu/
    ├── Cargo.toml
    └── src/lib.rs

userspace/filesystem/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── vfs.rs            # Virtual file system
│   ├── server.rs         # FS server IPC
│   ├── cache.rs          # Page cache
│   ├── store.rs         # Content-addressable
│   └── immutable.rs      # Immutable fs
└── mounts/
    └── rootfs.mount

userspace/network/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── stack.rs          # Network stack
│   ├── ethernet.rs       # Eth layer
│   ├── ipv4.rs           # IPv4
│   ├── tcp.rs            # TCP
│   ├── udp.rs            # UDP
│   └── socket.rs         # Socket API
```

### Dependencies
- **Input**: Driver Manager (Track 7), Virtio (Phase 8.1)
- **Output**: Full system with storage and network

---

# PARALLEL EXECUTION MATRIX

## Dependency Graph

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         PARALLEL EXECUTION FLOW                             │
│                                                                             │
│  ┌─────────────┐                                                           │
│  │   Track 1   │  Bootloader & Kernel Entry                               │
│  │  (Agent 1)  │  - Complete independent                                   │
│  └──────┬──────┘                                                           │
│         │                                                                  │
│         │  BootInfo ──────▶ ┌─────────────┐                                │
│         └─────────────────▶ │   Track 2   │ Memory Manager (Agent 2)       │
│                             │  (Agent 2)  │  - Starts after Week 1          │
│                             └──────┬──────┘                                │
│                                    │                                        │
│                                    │ Mapper ─────▶ ┌──────────────────┐    │
│                                    └─────────────▶│     Track 3      │    │
│                                                   │    Scheduler     │    │
│                                                   │    (Agent 3)     │    │
│                                                   └────────┬─────────┘    │
│                                                            │               │
│                              ┌─────────────────────────────┼───────────┐  │
│                              │                             │           │  │
│                    ┌─────────▼─────────┐     ┌──────────────▼──────────┐ │  │
│                    │     Track 4      │     │      Track 5            │ │  │
│                    │   Interrupt      │     │       IPC               │ │  │
│                    │   (Agent 4)      │     │      (Agent 5)          │ │  │
│                    └─────────┬────────┘     └───────────┬────────────┘ │  │
│                              │                         │                │  │
│                              └───────────┬─────────────┘                │  │
│                                          │                              │  │
│                                          ▼                              │  │
│                              ┌──────────────────────┐                  │  │
│                              │      Track 6         │                  │  │
│                              │    Capability        │                  │  │
│                              │     (Agent 6)        │                  │  │
│                              └──────────┬───────────┘                  │  │
│                                         │                              │  │
│                                         │ Capability + IPC            │  │
│                                         ▼                              │  │
│                              ┌──────────────────────┐                  │  │
│                              │      Track 7         │                  │  │
│                              │   Userspace Core     │                  │  │
│                              │      (Agent 7)       │                  │  │
│                              └──────────┬───────────┘                  │  │
│                                         │                              │  │
│                                         │ Init + Driver Manager       │  │
│                                         ▼                              │  │
│                              ┌──────────────────────┐                  │  │
│                              │      Track 8          │                  │  │
│                              │ Drivers + FS + Net    │                  │  │
│                              │       (Agent 8)       │                  │  │
│                              └───────────────────────┘                  │  │
└─────────────────────────────────────────────────────────────────────────┘
```

## Synchronization Points

### Month 1 (Week 1-4)
- **Agent 1**: Bootloader development (independent)
- **Agent 2**: Project setup, create kernel scaffolding
- **Agents 3-8**: Wait / assist Agent 1

### Month 2 (Week 5-8)
- **Agent 1**: Complete bootloader, kernel entry
- **Agent 2**: Memory management (parallel with Agent 1)
- **Agent 3**: Initial task structures
- **Agent 4**: Initial IDT setup
- **Agents 5-8**: Wait

### Month 3 (Week 9-12)
- **Agent 1**: Done - BootInfo available
- **Agent 2**: Complete memory management
- **Agent 3**: Scheduler implementation
- **Agent 4**: Interrupt handling
- **Agents 5-8**: Start development (can use mocks)

### Month 4 (Week 13-16)
- **Agents 1-2**: Integration testing
- **Agent 3**: Complete scheduler
- **Agent 4**: Complete interrupts
- **Agent 5**: IPC system
- **Agent 6**: Capability design
- **Agent 7**: Syscall interface
- **Agent 8**: Driver framework design

### Month 5+ (Parallel Execution)
- All agents work in parallel
- Weekly sync meetings
- Integration builds every 2 weeks

---

# INTER-TRACK INTERFACES

## Interface Definitions

### 1. Bootloader → Kernel (Track 1 → 2)
```rust
#[repr(C)]
pub struct BootInfo {
    pub magic: u64,              // 0x53494752
    pub version: u32,
    pub memory_map: PhysAddr,
    pub memory_map_size: usize,
    pub kernel_phys_start: PhysAddr,
    pub kernel_virt_start: PhysAddr,
    pub kernel_size: usize,
}
```

### 2. Memory → Scheduler (Track 2 → 3)
```rust
pub trait MemoryAllocator: Send + Sync {
    fn allocate(&self, size: usize, align: usize) -> Result<VirtAddr, AllocError>;
    fn deallocate(&self, addr: VirtAddr, size: usize);
}
```

### 3. Memory → IPC (Track 2 → 5)
```rust
pub trait PageTable: Send + Sync {
    fn map(&mut self, virt: VirtAddr, phys: PhysFrame, flags: PageFlags) -> Result<()>;
    fn unmap(&mut self, virt: VirtAddr) -> Result<PhysFrame, MapError>;
}
```

### 4. Scheduler → IPC (Track 3 → 5)
```rust
pub fn wake_task(task_id: TaskId);
pub fn sleep_until(deadline: Deadline) -> Result<(), SleepError>;
```

### 5. IPC → Capability (Track 5 → 6)
```rust
pub fn transfer_capability(cap: CapabilityId, target: ProcessId) -> Result<CapabilitySlot, CapError>;
pub fn insert_capability(process: ProcessId, cap: Capability) -> Result<CapabilitySlot, CapError>;
```

### 6. Kernel → Userspace (Track 7)
```rust
// Syscall numbers in libs/syscall-api/src/number.rs
// Capability rights in userspace/common/src/cap.rs
```

---

# TESTING STRATEGY PER TRACK

| Track | Unit Tests | Integration Tests | Tools |
|-------|------------|-------------------|-------|
| 1 (Boot) | Memory parsing | QEMU boot | OVMF |
| 2 (Memory) | Allocator tests | Page fault tests | mmap stress |
| 3 (Sched) | Runqueue tests | Multi-core tests | perf |
| 4 (IRQ) | Handler tests | Interrupt latency | hwtrace |
| 5 (IPC) | Message tests | Cross-process | benchmark |
| 6 (Cap) | Derivation tests | Security tests | fuzz |
| 7 (Userspace) | Service tests | Full boot | QEMU |
| 8 (Drivers) | Driver tests | I/O tests | fio/iperf |

---

# BUILD & CI STRUCTURE

## Build Commands

```bash
# Build everything
cargo build --workspace

# Build individual tracks
cargo build -p boot              # Track 1
cargo build -p kernel            # Tracks 2-6
cargo build -p userspace        # Tracks 7-8

# Run tests
cargo test --workspace

# QEMU boot test
./build/scripts/qemu.sh --kernel build/kernel --initrd build/initramfs
```

## CI Pipeline

```yaml
# .github/workflows/ci.yml
name: CI
on: [push, pull_request]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - name: Build
        run: cargo build --workspace --release
      - name: Test
        run: cargo test --workspace
      - name: Clippy
        run: cargo clippy --workspace
      - name: Miri
        run: cargo miri test

  qemu-test:
    needs: build
    runs-on: ubuntu-latest
    steps:
      - name: Run QEMU tests
        run: ./build/scripts/test-qemu.sh
```

---

# SUCCESS CRITERIA

## Phase 1 Complete (End of Month 3)
- [ ] Kernel boots via UEFI
- [ ] Basic memory allocation works
- [ ] Tasks can be created and scheduled
- [ ] Interrupts handled
- [ ] Shell boots

## Phase 2 Complete (End of Month 6)
- [ ] Full IPC system works
- [ ] Capability model enforced
- [ ] Init service manages processes
- [ ] Drivers load

## Phase 3 Complete (End of Month 12)
- [ ] Filesystem operational
- [ ] Network stack works
- [ ] Immutable root implemented
- [ ] Update system functional

## Production Ready (End of Month 18)
- [ ] Full test coverage
- [ ] Performance targets met
- [ ] Cloud VM optimized
- [ ] Documentation complete
