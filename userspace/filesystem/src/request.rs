//! Filesystem request types
//!
//! Defines the protocol between clients and the filesystem server.

use common::ipc::Message;

pub const ENTRY_TYPE_FILE: u8 = 1;
pub const ENTRY_TYPE_DIR: u8 = 2;
pub const ENTRY_TYPE_SYMLINK: u8 = 3;
pub const ENTRY_TYPE_DEVICE: u8 = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FsRequestType {
    Open = 0,
    Read = 1,
    Write = 2,
    Close = 3,
    Stat = 4,
    Mkdir = 5,
    Unlink = 6,
    ReadDir = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FsResponseType {
    Ok = 0,
    Error = 1,
    Handle = 2,
    Stat = 3,
    DirEntry = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum FsErrorCode {
    Success = 0,
    NotFound = -1,
    AlreadyExists = -2,
    PermissionDenied = -3,
    InvalidPath = -4,
    NotDirectory = -5,
    IsDirectory = -6,
    IoError = -7,
    NotSupported = -8,
    InvalidHandle = -9,
    MountFailed = -10,
    BufferTooSmall = -11,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FsResponseHeader {
    pub resp_type: FsResponseType,
    pub error_code: FsErrorCode,
    pub payload_len: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OpenRequest {
    pub flags: u32,
    pub mode: u32,
    pub path_len: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct OpenResponse {
    pub handle: u64,
    pub size: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ReadRequest {
    pub handle: u64,
    pub offset: u64,
    pub size: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ReadResponse {
    pub bytes_read: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WriteRequest {
    pub handle: u64,
    pub offset: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct WriteResponse {
    pub bytes_written: u32,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct CloseRequest {
    pub handle: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StatRequest {
    pub path_len: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct StatResponse {
    pub size: u64,
    pub mode: u32,
    pub created: u64,
    pub modified: u64,
    pub accessed: u64,
    pub is_dir: u8,
    pub is_file: u8,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct MkdirRequest {
    pub mode: u32,
    pub path_len: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct UnlinkRequest {
    pub path_len: u16,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ReadDirRequest {
    pub handle: u64,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DirEntryInfo {
    pub name_len: u16,
    pub entry_type: u8,
    pub _pad: u8,
    pub size: u64,
}
