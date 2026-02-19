//! System call argument structures

/// System call arguments
/// 
/// This is passed to the syscall instruction with each argument
/// in a specific register.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SyscallArgs {
    pub num: u64,
    pub arg0: u64,
    pub arg1: u64,
    pub arg2: u64,
    pub arg3: u64,
    pub arg4: u64,
    pub arg5: u64,
}

impl SyscallArgs {
    /// Create a new syscall with no arguments
    #[inline]
    pub const fn new(num: u64) -> Self {
        Self {
            num,
            arg0: 0,
            arg1: 0,
            arg2: 0,
            arg3: 0,
            arg4: 0,
            arg5: 0,
        }
    }
    
    /// Create a syscall with 1 argument
    #[inline]
    pub const fn with_arg0(mut self, arg0: u64) -> Self {
        self.arg0 = arg0;
        self
    }
    
    /// Create a syscall with 2 arguments
    #[inline]
    pub const fn with_args(mut self, arg0: u64, arg1: u64) -> Self {
        self.arg0 = arg0;
        self.arg1 = arg1;
        self
    }
    
    /// Create a syscall with 3 arguments
    #[inline]
    pub const fn with_3args(mut self, arg0: u64, arg1: u64, arg2: u64) -> Self {
        self.arg0 = arg0;
        self.arg1 = arg1;
        self.arg2 = arg2;
        self
    }
    
    /// Create a syscall with 4 arguments
    #[inline]
    pub const fn with_4args(mut self, arg0: u64, arg1: u64, arg2: u64, arg3: u64) -> Self {
        self.arg0 = arg0;
        self.arg1 = arg1;
        self.arg2 = arg2;
        self.arg3 = arg3;
        self
    }
}
