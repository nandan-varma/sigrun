//! Immutable Filesystem Implementation
//!
//! Implements a simple in-memory filesystem for SIGRUN.

use crate::common::handle::{FileHandle, OpenFlags};
use crate::vfs::{FileStat, FileSystem, FsError};

pub struct ImmutableFs {
    root_dir: DirEntrySimple,
}

struct DirEntrySimple {
    name: [u8; 32],
    name_len: usize,
    is_dir: bool,
}

impl Default for ImmutableFs {
    fn default() -> Self {
        Self::new()
    }
}

impl ImmutableFs {
    pub const fn new() -> Self {
        let mut name = [0u8; 32];
        name[0] = b'/';
        Self {
            root_dir: DirEntrySimple {
                name,
                name_len: 1,
                is_dir: true,
            },
        }
    }
}

impl FileSystem for ImmutableFs {
    fn mount(&mut self, _path: &str) -> Result<(), FsError> {
        Ok(())
    }

    fn unmount(&mut self) -> Result<(), FsError> {
        Ok(())
    }

    fn open(&self, path: &str, flags: OpenFlags) -> Result<FileHandle, FsError> {
        if flags.contains(OpenFlags::WRITE) {
            return Err(FsError::PermissionDenied);
        }

        let _ = path;

        Ok(FileHandle::new(core::num::NonZeroU64::new(100).unwrap()))
    }

    fn close(&self, _handle: FileHandle) -> Result<(), FsError> {
        Ok(())
    }

    fn read(&self, _handle: FileHandle, _offset: u64, buf: &mut [u8]) -> Result<usize, FsError> {
        let welcome = b"Welcome to SIGRUN!\nThis is an immutable filesystem.\n";
        let len = welcome.len().min(buf.len());
        buf[..len].copy_from_slice(&welcome[..len]);
        Ok(len)
    }

    fn write(&self, _handle: FileHandle, _offset: u64, _data: &[u8]) -> Result<usize, FsError> {
        Err(FsError::PermissionDenied)
    }

    fn stat(&self, path: &str) -> Result<FileStat, FsError> {
        let _ = path;

        Ok(FileStat {
            size: 0,
            mode: 0o555,
            created: 0,
            modified: 0,
            accessed: 0,
            is_dir: true,
            is_file: false,
        })
    }

    fn mkdir(&mut self, _path: &str) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }

    fn unlink(&mut self, _path: &str) -> Result<(), FsError> {
        Err(FsError::PermissionDenied)
    }
}
