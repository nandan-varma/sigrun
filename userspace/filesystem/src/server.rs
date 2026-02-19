//! Filesystem server implementation
//!
//! Handles incoming IPC requests and routes them to the appropriate
//! filesystem backend.

use crate::request::*;
use crate::vfs::Vfs;
use common::handle::FileHandle;
use common::ipc::Message;

pub struct FsServer {
    vfs: Vfs,
}

impl FsServer {
    pub const fn new(vfs: Vfs) -> Self {
        Self { vfs }
    }

    pub fn run(&mut self) -> ! {
        loop {
            self.tick();
        }
    }

    fn tick(&mut self) {
        crate::yield_now();
    }

    fn handle_request(&mut self, msg: &Message) -> Message {
        if msg.header.payload_len < 1 {
            return build_error_response(FsErrorCode::InvalidPath);
        }

        let req_type = match msg.payload.get(0) {
            Some(&0) => FsRequestType::Open,
            Some(&1) => FsRequestType::Read,
            Some(&2) => FsRequestType::Write,
            Some(&3) => FsRequestType::Close,
            Some(&4) => FsRequestType::Stat,
            Some(&5) => FsRequestType::Mkdir,
            Some(&6) => FsRequestType::Unlink,
            Some(&7) => FsRequestType::ReadDir,
            _ => return build_error_response(FsErrorCode::NotSupported),
        };

        match req_type {
            FsRequestType::Open => self.handle_open(msg),
            FsRequestType::Read => self.handle_read(msg),
            FsRequestType::Write => self.handle_write(msg),
            FsRequestType::Close => self.handle_close(msg),
            FsRequestType::Stat => self.handle_stat(msg),
            FsRequestType::Mkdir => self.handle_mkdir(msg),
            FsRequestType::Unlink => self.handle_unlink(msg),
            FsRequestType::ReadDir => self.handle_readdir(msg),
        }
    }

    fn handle_open(&mut self, msg: &Message) -> Message {
        let req_size = core::mem::size_of::<OpenRequest>();
        if (msg.header.payload_len as usize) <= req_size {
            return build_error_response(FsErrorCode::InvalidPath);
        }

        let req: &OpenRequest = unsafe { &*(msg.payload.as_ptr() as *const OpenRequest) };

        let path_start = req_size;
        let path_end = path_start + req.path_len as usize;

        if path_end > msg.header.payload_len as usize {
            return build_error_response(FsErrorCode::InvalidPath);
        }

        let path = unsafe { core::str::from_utf8_unchecked(&msg.payload[path_start..path_end]) };

        let flags = common::handle::OpenFlags::from_bits_truncate(req.flags);

        match self.vfs.open(path, flags) {
            Ok(handle) => build_open_response(handle.raw(), 0),
            Err(crate::vfs::FsError::NotFound) => build_error_response(FsErrorCode::NotFound),
            Err(crate::vfs::FsError::PermissionDenied) => {
                build_error_response(FsErrorCode::PermissionDenied)
            }
            Err(_) => build_error_response(FsErrorCode::IoError),
        }
    }

    fn handle_read(&mut self, msg: &Message) -> Message {
        let req_size = core::mem::size_of::<ReadRequest>();
        if (msg.header.payload_len as usize) < req_size {
            return build_error_response(FsErrorCode::InvalidHandle);
        }

        let req: &ReadRequest = unsafe { &*(msg.payload.as_ptr() as *const ReadRequest) };

        let handle = match FileHandle::from_raw(req.handle) {
            Some(h) => h,
            None => return build_error_response(FsErrorCode::InvalidHandle),
        };

        let mut buffer = [0u8; 256];
        let to_read = req.size.min(buffer.len() as u32) as usize;

        match self.vfs.read(handle, &mut buffer[..to_read]) {
            Ok(bytes_read) => build_read_response(&buffer[..bytes_read], bytes_read as u32),
            Err(_) => build_error_response(FsErrorCode::IoError),
        }
    }

    fn handle_write(&mut self, msg: &Message) -> Message {
        let req_size = core::mem::size_of::<WriteRequest>();
        if (msg.header.payload_len as usize) < req_size {
            return build_error_response(FsErrorCode::InvalidHandle);
        }

        let req: &WriteRequest = unsafe { &*(msg.payload.as_ptr() as *const WriteRequest) };

        let handle = match FileHandle::from_raw(req.handle) {
            Some(h) => h,
            None => return build_error_response(FsErrorCode::InvalidHandle),
        };

        let data = &msg.payload[req_size..msg.header.payload_len as usize];

        match self.vfs.write(handle, data) {
            Ok(bytes_written) => build_write_response(bytes_written as u32),
            Err(_) => build_error_response(FsErrorCode::IoError),
        }
    }

    fn handle_close(&mut self, msg: &Message) -> Message {
        let req_size = core::mem::size_of::<CloseRequest>();
        if (msg.header.payload_len as usize) < req_size {
            return build_error_response(FsErrorCode::InvalidHandle);
        }

        let req: &CloseRequest = unsafe { &*(msg.payload.as_ptr() as *const CloseRequest) };

        let handle = match FileHandle::from_raw(req.handle) {
            Some(h) => h,
            None => return build_error_response(FsErrorCode::InvalidHandle),
        };

        match self.vfs.close(handle) {
            Ok(_) => build_ok_response(),
            Err(_) => build_error_response(FsErrorCode::IoError),
        }
    }

    fn handle_stat(&mut self, msg: &Message) -> Message {
        let req_size = core::mem::size_of::<StatRequest>();
        if (msg.header.payload_len as usize) <= req_size {
            return build_error_response(FsErrorCode::InvalidPath);
        }

        let req: &StatRequest = unsafe { &*(msg.payload.as_ptr() as *const StatRequest) };

        let path_start = req_size;
        let path_end = path_start + req.path_len as usize;

        if path_end > msg.header.payload_len as usize {
            return build_error_response(FsErrorCode::InvalidPath);
        }

        let _path = unsafe { core::str::from_utf8_unchecked(&msg.payload[path_start..path_end]) };

        match self.vfs.stat(_path) {
            Ok(stat) => build_stat_response(
                stat.size,
                stat.mode,
                stat.created,
                stat.modified,
                stat.accessed,
                stat.is_dir as u8,
                stat.is_file as u8,
            ),
            Err(_) => build_error_response(FsErrorCode::IoError),
        }
    }

    fn handle_mkdir(&mut self, msg: &Message) -> Message {
        let req_size = core::mem::size_of::<MkdirRequest>();
        if (msg.header.payload_len as usize) <= req_size {
            return build_error_response(FsErrorCode::InvalidPath);
        }

        match self.vfs.mkdir("") {
            Ok(_) => build_ok_response(),
            Err(_) => build_error_response(FsErrorCode::IoError),
        }
    }

    fn handle_unlink(&mut self, msg: &Message) -> Message {
        let req_size = core::mem::size_of::<UnlinkRequest>();
        if (msg.header.payload_len as usize) <= req_size {
            return build_error_response(FsErrorCode::InvalidPath);
        }

        match self.vfs.unlink("") {
            Ok(_) => build_ok_response(),
            Err(_) => build_error_response(FsErrorCode::IoError),
        }
    }

    fn handle_readdir(&mut self, _msg: &Message) -> Message {
        build_error_response(FsErrorCode::NotSupported)
    }
}

fn build_error_response(code: FsErrorCode) -> Message {
    let mut msg = Message::call();

    let resp = FsResponseHeader {
        resp_type: FsResponseType::Error,
        error_code: code,
        payload_len: core::mem::size_of::<FsResponseHeader>() as u16,
    };

    let resp_bytes = unsafe {
        core::slice::from_raw_parts(
            &resp as *const FsResponseHeader as *const u8,
            core::mem::size_of::<FsResponseHeader>(),
        )
    };

    msg.payload[..resp_bytes.len()].copy_from_slice(resp_bytes);
    msg.header.payload_len = resp_bytes.len() as u16;
    msg
}

fn build_ok_response() -> Message {
    build_error_response(FsErrorCode::Success)
}

fn build_open_response(handle: u64, size: u64) -> Message {
    let mut msg = Message::call();

    let resp = OpenResponse { handle, size };

    let resp_bytes = unsafe {
        core::slice::from_raw_parts(
            &resp as *const OpenResponse as *const u8,
            core::mem::size_of::<OpenResponse>(),
        )
    };

    msg.payload[..resp_bytes.len()].copy_from_slice(resp_bytes);
    msg.header.payload_len = resp_bytes.len() as u16;
    msg
}

fn build_read_response(data: &[u8], bytes_read: u32) -> Message {
    let mut msg = Message::call();

    let resp = ReadResponse { bytes_read };

    let resp_bytes = unsafe {
        core::slice::from_raw_parts(
            &resp as *const ReadResponse as *const u8,
            core::mem::size_of::<ReadResponse>(),
        )
    };

    let data_len = data.len().min(256 - resp_bytes.len());

    msg.payload[..resp_bytes.len()].copy_from_slice(resp_bytes);
    msg.payload[resp_bytes.len()..resp_bytes.len() + data_len].copy_from_slice(&data[..data_len]);
    msg.header.payload_len = (resp_bytes.len() + data_len) as u16;
    msg
}

fn build_write_response(bytes_written: u32) -> Message {
    let mut msg = Message::call();

    let resp = WriteResponse { bytes_written };

    let resp_bytes = unsafe {
        core::slice::from_raw_parts(
            &resp as *const WriteResponse as *const u8,
            core::mem::size_of::<WriteResponse>(),
        )
    };

    msg.payload[..resp_bytes.len()].copy_from_slice(resp_bytes);
    msg.header.payload_len = resp_bytes.len() as u16;
    msg
}

fn build_stat_response(
    size: u64,
    mode: u32,
    created: u64,
    modified: u64,
    accessed: u64,
    is_dir: u8,
    is_file: u8,
) -> Message {
    let mut msg = Message::call();

    let resp = StatResponse {
        size,
        mode,
        created,
        modified,
        accessed,
        is_dir,
        is_file,
    };

    let resp_bytes = unsafe {
        core::slice::from_raw_parts(
            &resp as *const StatResponse as *const u8,
            core::mem::size_of::<StatResponse>(),
        )
    };

    msg.payload[..resp_bytes.len()].copy_from_slice(resp_bytes);
    msg.header.payload_len = resp_bytes.len() as u16;
    msg
}
