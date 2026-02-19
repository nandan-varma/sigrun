//! System call numbers

/// Process management
pub const SYSCALL_EXIT: u64 = 0;
pub const SYSCALL_FORK: u64 = 1;
pub const SYSCALL_EXEC: u64 = 2;
pub const SYSCALL_WAIT: u64 = 3;
pub const SYSCALL_GETPID: u64 = 4;
pub const SYSCALL_GETPPID: u64 = 5;

/// Memory
pub const SYSCALL_MMAP: u64 = 10;
pub const SYSCALL_MUNMAP: u64 = 11;
pub const SYSCALL_MPROTECT: u64 = 12;

/// File I/O
pub const SYSCALL_OPEN: u64 = 20;
pub const SYSCALL_CLOSE: u64 = 21;
pub const SYSCALL_READ: u64 = 22;
pub const SYSCALL_WRITE: u64 = 23;
pub const SYSCALL_STAT: u64 = 24;
pub const SYSCALL_LSEEK: u64 = 25;

/// IPC
pub const SYSCALL_IPC_SEND: u64 = 30;
pub const SYSCALL_IPC_RECV: u64 = 31;
pub const SYSCALL_IPC_CREATE: u64 = 32;
pub const SYSCALL_IPC_DESTROY: u64 = 33;

/// Capabilities
pub const SYSCALL_CAP_GRANT: u64 = 40;
pub const SYSCALL_CAP_REVOKE: u64 = 41;
pub const SYSCALL_CAP_DERIVE: u64 = 42;
pub const SYSCALL_CAP_LOOKUP: u64 = 43;

/// Scheduling
pub const SYSCALL_YIELD: u64 = 50;
pub const SYSCALL_SET_PRIORITY: u64 = 51;
pub const SYSCALL_GET_PRIORITY: u64 = 52;
pub const SYSCALL_SLEEP: u64 = 53;
pub const SYSCALL_WAKE: u64 = 54;

/// Time
pub const SYSCALL_GETTIME: u64 = 60;
pub const SYSCALL_CLOCK_GETTIME: u64 = 61;

/// Process info
pub const SYSCALL_GETCPU: u64 = 70;
pub const SYSCALL_GETUID: u64 = 71;
