//! SIGRUN Init Service (PID 1)
//!
//! This is the first userspace process that starts after kernel boot.
//! It is responsible for:
//! 1. Receiving initial capabilities from kernel
//! 2. Parsing service manifest
//! 3. Starting system services
//! 4. Managing service lifecycle

#![no_std]
#![no_main]

use core::panic::PanicInfo;
use syscall_api::{SYSCALL_WRITE, SyscallArgs};

const MAX_SERVICES: usize = 8;
const MAX_NAME_LEN: usize = 32;
const MAX_CAPS: usize = 4;

/// Main entry point for init service
#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    main()
}

fn main() -> ! {
    print("SIGRUN Init Service v0.1\n");
    print("========================\n\n");

    let caps = get_initial_capabilities();
    print("Received initial capabilities\n");

    let mut manager = ServiceManager::new(caps);

    print("Loading service manifest...\n");
    match manager.load_manifest() {
        Ok(manifest) => {
            print("Manifest loaded: ");
            print_u64(manifest.service_count as u64);
            print(" services\n");

            print("Starting services...\n\n");
            if let Err(_) = manager.start_services(&manifest) {
                print("ERROR: Failed to start services\n");
            }
        }
        Err(_) => {
            print("WARNING: Could not load manifest, starting minimal system\n");
        }
    }

    print("\nInit complete. Entering service loop...\n");
    service_loop();
}

/// Print a string using syscall
fn print(s: &str) {
    let args = SyscallArgs::new(SYSCALL_WRITE).with_3args(1, s.as_ptr() as u64, s.len() as u64);
    unsafe {
        let _ = syscall_api::syscall(args);
    }
}

/// Print a u64
fn print_u64(n: u64) {
    if n == 0 {
        print("0");
        return;
    }

    let mut buf = [0u8; 20];
    let mut i = 20;
    let mut n = n;

    while n > 0 {
        i -= 1;
        buf[i] = b'0' + (n % 10) as u8;
        n /= 10;
    }

    print(core::str::from_utf8(&buf[i..]).unwrap_or("?"));
}

/// Get initial capabilities from kernel
fn get_initial_capabilities() -> CapabilitySet {
    CapabilitySet::new()
}

/// Fixed-size capability set
struct CapabilitySet {
    caps: [Capability; MAX_CAPS],
    count: usize,
}

impl CapabilitySet {
    const fn new() -> Self {
        Self {
            caps: [Capability { id: 0, rights: 0 }; MAX_CAPS],
            count: 0,
        }
    }

    fn len(&self) -> usize {
        self.count
    }
}

/// Service Manager
struct ServiceManager {
    capabilities: CapabilitySet,
    services: [Service; MAX_SERVICES],
    service_count: usize,
}

#[derive(Clone, Copy)]
struct Service {
    name: [u8; MAX_NAME_LEN],
    pid: u64,
    state: ServiceState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ServiceState {
    Starting,
    Running,
    Stopped,
    Failed,
}

impl ServiceManager {
    const fn new(caps: CapabilitySet) -> Self {
        Self {
            capabilities: caps,
            services: [Service {
                name: [0; MAX_NAME_LEN],
                pid: 0,
                state: ServiceState::Stopped,
            }; MAX_SERVICES],
            service_count: 0,
        }
    }

    /// Load service manifest from file
    fn load_manifest(&mut self) -> Result<ServiceManifest, ManifestError> {
        let mut manifest = ServiceManifest::new();

        // Add driver-manager service
        let name = b"driver-manager";
        let prog = b"/sbin/driver-manager";

        let mut def = ServiceDef {
            name: [0; MAX_NAME_LEN],
            program: [0; MAX_NAME_LEN],
            capabilities: [Capability { id: 0, rights: 0 }; MAX_CAPS],
        };

        def.name[..name.len()].copy_from_slice(name);
        def.program[..prog.len()].copy_from_slice(prog);
        def.capabilities[0] = Capability { id: 1, rights: 0b1 };

        manifest.add(def)?;
        Ok(manifest)
    }

    /// Start all services in manifest
    fn start_services(&mut self, manifest: &ServiceManifest) -> Result<(), StartError> {
        for i in 0..manifest.service_count {
            let def = &manifest.services[i];
            let name = core::str::from_utf8(&def.name)
                .unwrap_or("unknown")
                .trim_end_matches('\0');
            print("Starting: ");
            print(name);
            print("...\n");

            if self.service_count < MAX_SERVICES {
                self.services[self.service_count] = Service {
                    name: def.name,
                    pid: 0,
                    state: ServiceState::Running,
                };
                self.service_count += 1;
            }
        }
        Ok(())
    }
}

/// Service manifest
struct ServiceManifest {
    services: [ServiceDef; MAX_SERVICES],
    service_count: usize,
}

impl ServiceManifest {
    const fn new() -> Self {
        Self {
            services: [ServiceDef {
                name: [0; MAX_NAME_LEN],
                program: [0; MAX_NAME_LEN],
                capabilities: [Capability { id: 0, rights: 0 }; MAX_CAPS],
            }; MAX_SERVICES],
            service_count: 0,
        }
    }

    fn add(&mut self, def: ServiceDef) -> Result<(), ManifestError> {
        if self.service_count >= MAX_SERVICES {
            return Err(ManifestError::TooManyServices);
        }
        self.services[self.service_count] = def;
        self.service_count += 1;
        Ok(())
    }
}

/// Service definition from manifest
#[derive(Clone, Copy)]
struct ServiceDef {
    name: [u8; MAX_NAME_LEN],
    program: [u8; MAX_NAME_LEN],
    capabilities: [Capability; MAX_CAPS],
}

/// Capability placeholder
#[derive(Debug, Clone, Copy)]
struct Capability {
    id: u64,
    rights: u32,
}

/// Manifest errors
#[derive(Debug)]
enum ManifestError {
    NotFound,
    ParseError,
    TooManyServices,
}

/// Service start errors
#[derive(Debug)]
enum StartError {
    SpawnFailed,
    CapabilityError,
}

/// Main service loop
fn service_loop() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}

/// Panic handler
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    print("PANIC in init\n");
    loop {}
}
