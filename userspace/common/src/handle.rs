//! Handle types for resources

use core::num::NonZeroU64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Handle(NonZeroU64);

impl Handle {
    pub const fn new(raw: NonZeroU64) -> Self {
        Self(raw)
    }

    pub const fn raw(self) -> u64 {
        self.0.get()
    }

    pub const fn as_usize(self) -> usize {
        self.0.get() as usize
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileHandle(Handle);

impl FileHandle {
    pub const STDIN: Self = Self(Handle(NonZeroU64::new(0).unwrap()));
    pub const STDOUT: Self = Self(Handle(NonZeroU64::new(1).unwrap()));
    pub const STDERR: Self = Self(Handle(NonZeroU64::new(2).unwrap()));

    pub const fn new(raw: NonZeroU64) -> Self {
        Self(Handle::new(raw))
    }

    pub fn from_raw(raw: u64) -> Option<Self> {
        NonZeroU64::new(raw).map(Self::new)
    }

    pub const fn raw(self) -> u64 {
        self.0.raw()
    }

    pub const fn is_std(self) -> bool {
        matches!(self.0.raw(), 0..=2)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SocketHandle(Handle);

impl SocketHandle {
    pub const fn new(raw: NonZeroU64) -> Self {
        Self(Handle::new(raw))
    }

    pub fn from_raw(raw: u64) -> Option<Self> {
        NonZeroU64::new(raw).map(Self::new)
    }

    pub const fn raw(self) -> u64 {
        self.0.raw()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeviceHandle(Handle);

impl DeviceHandle {
    pub const fn new(raw: NonZeroU64) -> Self {
        Self(Handle::new(raw))
    }

    pub fn from_raw(raw: u64) -> Option<Self> {
        NonZeroU64::new(raw).map(Self::new)
    }

    pub const fn raw(self) -> u64 {
        self.0.raw()
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: u32 {
        const READ = 0b00000001;
        const WRITE = 0b00000010;
        const EXECUTE = 0b00000100;
        const CREATE = 0b00001000;
        const EXCLUSIVE = 0b00010000;
        const TRUNCATE = 0b00100000;
        const APPEND = 0b01000000;
        const DIRECTORY = 0b10000000;
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct FileMode: u32 {
        const OWNER_READ = 0o400;
        const OWNER_WRITE = 0o200;
        const OWNER_EXEC = 0o100;
        const GROUP_READ = 0o040;
        const GROUP_WRITE = 0o020;
        const GROUP_EXEC = 0o010;
        const OTHER_READ = 0o004;
        const OTHER_WRITE = 0o002;
        const OTHER_EXEC = 0o001;
        const SET_UID = 0o4000;
        const SET_GID = 0o2000;
        const STICKY = 0o1000;
    }
}
