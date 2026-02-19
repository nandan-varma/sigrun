//! EFI Utility Functions
//!
//! This module provides utility functions that work with the uefi crate
//! to perform common boot-time operations.

use uefi::Guid;

/// GUID for ACPI 2.0 RSDP
const ACPI_20_TABLE_GUID: Guid = Guid::new(
    [0x88, 0x68, 0xe8, 0x71],
    [0xe4, 0xf1],
    [0x11, 0xd3],
    0xbc,
    0x22,
    [0x00, 0x80, 0xc7, 0x3c, 0x88, 0x81],
);

/// GUID for ACPI 1.0 RSDP (for older systems)
const ACPI_TABLE_GUID: Guid = Guid::new(
    [0xeb, 0x9d, 0x2d, 0x30],
    [0x2d, 0x88],
    [0x11, 0xd3],
    0x9a,
    0x16,
    [0x00, 0x90, 0x27, 0x3f, 0xc1, 0x4d],
);

/// Find the ACPI RSDP (Root System Description Pointer)
///
/// Searches through UEFI configuration tables to find the ACPI
/// RSDP table, which contains the root of the ACPI tables.
/// Returns the physical address of the RSDP.
pub fn find_rsdp() -> Option<u64> {
    uefi::system::with_config_table(|tables| {
        // First try to find ACPI 2.0+ RSDP
        for table in tables {
            if table.guid == ACPI_20_TABLE_GUID {
                return Some(table.address as u64);
            }
        }

        // Fallback to ACPI 1.0 RSDP
        for table in tables {
            if table.guid == ACPI_TABLE_GUID {
                return Some(table.address as u64);
            }
        }

        None
    })
}

/// Get the EFI system table physical address
///
/// This is useful for the kernel to access EFI runtime services
/// after boot services are exited.
pub fn get_system_table_address() -> u64 {
    // In no_std without access to the system table, we return 0
    // The kernel will need to find this through other means
    0
}
