//! UEFI bindings and console I/O

use core::fmt;

/// UEFI Status code
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct Status(pub u64);

impl Status {
    pub const SUCCESS: Status = Status(0);
    pub const ABORTED: Status = Status(1);
    pub const LOAD_ERROR: Status = Status(2);
    pub const INVALID_PARAMETER: Status = Status(3);
    pub const OUT_OF_RESOURCES: Status = Status(4);
    pub const NOT_FOUND: Status = Status(5);
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UEFI Status: {:#x}", self.0)
    }
}

/// UEFI Handle
#[derive(Debug, Clone, Copy)]
#[repr(transparent)]
pub struct Handle(pub *mut core::ffi::c_void);

/// Simple UEFI System Table
#[repr(C)]
pub struct SystemTable {
    pub header: TableHeader,
    pub console_in_handle: Handle,
    pub console_in: *const SimpleTextInput,
    pub console_out: *mut SimpleTextOutput,
    pub stderr: *mut SimpleTextOutput,
    pub runtime: *mut RuntimeServices,
    pub boot: *mut BootServices,
}

#[repr(C)]
pub struct TableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

#[repr(C)]
pub struct SimpleTextInput {
    pub reset: extern "win64" fn(*mut SimpleTextInput, bool) -> Status,
    pub read_key: extern "win64" fn(*mut SimpleTextInput) -> Status,
}

#[repr(C)]
pub struct SimpleTextOutput {
    pub reset: extern "win64" fn(*mut SimpleTextOutput, bool) -> Status,
    pub output_string: extern "win64" fn(*mut SimpleTextOutput, *const u16) -> Status,
    pub test_string: extern "win64" fn(*mut SimpleTextOutput, *const u16) -> Status,
}

#[repr(C)]
pub struct RuntimeServices {
    // Simplified - real implementation would have more
    pub header: TableHeader,
}

#[repr(C)]
pub struct BootServices {
    pub header: TableHeader,
    // Many more function pointers would go here
}

impl SystemTable {
    pub fn stdout(&mut self) -> ConsoleWriter {
        unsafe { ConsoleWriter(self.console_out.as_mut().unwrap()) }
    }
}

pub struct ConsoleWriter(*mut SimpleTextOutput);

impl ConsoleWriter {
    pub fn write_str(&mut self, s: &str) -> Result<(), Status> {
        let mut buf: [u16; 256] = [0; 256];
        
        for (i, c) in s.encode_utf16().enumerate() {
            if i >= 255 { break; }
            buf[i] = c;
        }
        buf[s.len()] = 0;
        
        let status = unsafe {
            ((*self.0).output_string)(self.0, buf.as_ptr())
        };
        
        if status == Status::SUCCESS {
            Ok(())
        } else {
            Err(status)
        }
    }
}

impl core::fmt::Write for ConsoleWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        self.write_str(s).map_err(|_| core::fmt::Error)
    }
}

/// Find RSDP (Root System Description Pointer) from UEFI config tables
pub fn find_rsdp(st: &mut SystemTable) -> u64 {
    // Simplified: In real implementation, search EFI config tables
    // This is a placeholder that would need proper implementation
    0
}

/// Memory descriptor types
pub const MEMORY_TYPE_RESERVED: u32 = 0;
pub const MEMORY_TYPE_LOADER_CODE: u32 = 1;
pub const MEMORY_TYPE_LOADER_DATA: u32 = 2;
pub const MEMORY_TYPE_BOOT_SERVICES_CODE: u32 = 3;
pub const MEMORY_TYPE_BOOT_SERVICES_DATA: u32 = 4;
pub const MEMORY_TYPE_RUNTIME_SERVICES_CODE: u32 = 5;
pub const MEMORY_TYPE_RUNTIME_SERVICES_DATA: u32 = 6;
pub const MEMORY_TYPE_CONVENTIONAL: u32 = 7;
pub const MEMORY_TYPE_UNUSABLE: u32 = 8;
pub const MEMORY_TYPE_PERSISTENT: u32 = 9;

#[repr(C)]
pub struct MemoryDescriptor {
    pub type_: u32,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub page_count: u64,
    pub attribute: u64,
}
