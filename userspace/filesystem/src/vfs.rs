//! Virtual File System (VFS) Layer
//!
//! Provides a unified interface for multiple filesystem implementations.

use crate::common::handle::{FileHandle, OpenFlags};
use core::num::NonZeroU64;

const MAX_MOUNTS: usize = 4;
const MAX_OPEN_FILES: usize = 16;
const MAX_PATH_LEN: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FsError {
    NotFound,
    AlreadyExists,
    PermissionDenied,
    InvalidPath,
    NotDirectory,
    IsDirectory,
    IoError,
    NotSupported,
    InvalidHandle,
    MountFailed,
}

#[derive(Debug, Clone, Copy)]
pub struct FileStat {
    pub size: u64,
    pub mode: u32,
    pub created: u64,
    pub modified: u64,
    pub accessed: u64,
    pub is_dir: bool,
    pub is_file: bool,
}

impl FileStat {
    pub const fn new() -> Self {
        Self {
            size: 0,
            mode: 0,
            created: 0,
            modified: 0,
            accessed: 0,
            is_dir: false,
            is_file: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DirEntry {
    pub name: [u8; 64],
    pub name_len: usize,
    pub stat: FileStat,
}

pub trait FileSystem: Send {
    fn mount(&mut self, path: &str) -> Result<(), FsError>;
    fn unmount(&mut self) -> Result<(), FsError>;
    fn open(&self, path: &str, flags: OpenFlags) -> Result<FileHandle, FsError>;
    fn close(&self, handle: FileHandle) -> Result<(), FsError>;
    fn read(&self, handle: FileHandle, offset: u64, buf: &mut [u8]) -> Result<usize, FsError>;
    fn write(&self, handle: FileHandle, offset: u64, data: &[u8]) -> Result<usize, FsError>;
    fn stat(&self, path: &str) -> Result<FileStat, FsError>;
    fn mkdir(&mut self, path: &str) -> Result<(), FsError>;
    fn unlink(&mut self, path: &str) -> Result<(), FsError>;
}

pub struct Vfs {
    next_handle: u64,
    has_fs: bool,
}

impl Vfs {
    pub const fn new() -> Self {
        Self {
            next_handle: 3,
            has_fs: false,
        }
    }

    pub fn mount(&mut self, mount_path: &str, _fs: impl FileSystem) -> Result<(), FsError> {
        let _ = mount_path;
        self.has_fs = true;
        Ok(())
    }

    pub fn open(&self, path: &str, flags: OpenFlags) -> Result<FileHandle, FsError> {
        if !self.has_fs {
            return Err(FsError::NotFound);
        }

        let _ = (path, flags);

        Ok(FileHandle::new(NonZeroU64::new(100).unwrap()))
    }

    pub fn close(&self, _handle: FileHandle) -> Result<(), FsError> {
        Ok(())
    }

    pub fn read(&self, _handle: FileHandle, _buf: &mut [u8]) -> Result<usize, FsError> {
        Ok(0)
    }

    pub fn write(&self, _handle: FileHandle, _data: &[u8]) -> Result<usize, FsError> {
        Ok(0)
    }

    pub fn stat(&self, _path: &str) -> Result<FileStat, FsError> {
        Ok(FileStat::new())
    }

    pub fn mkdir(&mut self, _path: &str) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }

    pub fn unlink(&mut self, _path: &str) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }

    pub fn readdir(&self, _path: &str) -> Result<(), FsError> {
        Err(FsError::NotSupported)
    }

    pub fn read_handle_path(&self, _handle: FileHandle) -> Option<&str> {
        None
    }
}
