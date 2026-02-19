//! Driver framework

use crate::virtio::{VirtioBlkDriver, VirtioDevice, VirtioDriver};
use alloc::vec::Vec;

extern crate alloc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DriverType {
    VirtioBlk,
    VirtioNet,
    VirtioGpu,
    Unknown,
}

pub struct Driver {
    pub driver_type: DriverType,
    pub name: &'static str,
    data: DriverData,
}

enum DriverData {
    VirtioBlk(VirtioBlkDriver),
    VirtioNet,
    VirtioGpu,
    None,
}

impl Driver {
    pub fn new_virtio_blk() -> Self {
        Self {
            driver_type: DriverType::VirtioBlk,
            name: "virtio-blk",
            data: DriverData::VirtioBlk(VirtioBlkDriver::new()),
        }
    }

    pub fn new_virtio_net() -> Self {
        Self {
            driver_type: DriverType::VirtioNet,
            name: "virtio-net",
            data: DriverData::VirtioNet,
        }
    }

    pub fn new_virtio_gpu() -> Self {
        Self {
            driver_type: DriverType::VirtioGpu,
            name: "virtio-gpu",
            data: DriverData::VirtioGpu,
        }
    }

    pub fn init(&mut self) {
        match &mut self.data {
            DriverData::VirtioBlk(driver) => driver.init(),
            _ => {}
        }
    }
}

pub struct DriverRegistry {
    drivers: Vec<Driver>,
}

impl DriverRegistry {
    pub const fn new() -> Self {
        Self {
            drivers: Vec::new(),
        }
    }

    pub fn register(&mut self, driver_type: DriverType, device: VirtioDevice) {
        let driver = match driver_type {
            DriverType::VirtioBlk => Driver::new_virtio_blk(),
            DriverType::VirtioNet => Driver::new_virtio_net(),
            DriverType::VirtioGpu => Driver::new_virtio_gpu(),
            DriverType::Unknown => return,
        };

        self.drivers.push(driver);
    }

    pub fn get(&self, index: usize) -> Option<&Driver> {
        self.drivers.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Driver> {
        self.drivers.get_mut(index)
    }

    pub fn count(&self) -> usize {
        self.drivers.len()
    }

    pub fn find_by_type(&self, driver_type: DriverType) -> Option<&Driver> {
        self.drivers.iter().find(|d| d.driver_type == driver_type)
    }
}
