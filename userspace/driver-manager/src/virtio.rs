//! Virtio Driver Support

use crate::pci::PciDevice;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtioDeviceType {
    Invalid = 0,
    Network = 1,
    Block = 2,
    Console = 3,
    Entropy = 4,
    MemoryBalloon = 5,
    IoThread = 6,
    Gpu = 16,
    Input = 18,
    Vsock = 19,
}

impl From<u8> for VirtioDeviceType {
    fn from(value: u8) -> Self {
        match value {
            1 => Self::Network,
            2 => Self::Block,
            3 => Self::Console,
            4 => Self::Entropy,
            5 => Self::MemoryBalloon,
            6 => Self::IoThread,
            16 => Self::Gpu,
            18 => Self::Input,
            19 => Self::Vsock,
            _ => Self::Invalid,
        }
    }
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VirtioFeatures: u64 {
        const VIRTIO_F_RING_INDIRECT_DESC = 1 << 28;
        const VIRTIO_F_RING_EVENT_IDX = 1 << 29;
        const VIRTIO_F_VERSION_1 = 1 << 32;
    }
}

#[derive(Debug, Clone, Copy)]
pub struct VirtioDevice {
    pub device_type: VirtioDeviceType,
    pub features: VirtioFeatures,
    pub status: VirtioStatus,
    pub queue_num: u16,
    pub config: VirtioConfig,
}

#[derive(Debug, Clone, Copy)]
pub struct VirtioConfig {
    pub base_addr: u64,
    pub interrupt: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtioStatus {
    pub acknowledged: bool,
    pub driver: bool,
    pub driver_ok: bool,
    pub features_ok: bool,
    pub failed: bool,
}

impl VirtioStatus {
    pub const fn new() -> Self {
        Self {
            acknowledged: false,
            driver: false,
            driver_ok: false,
            features_ok: false,
            failed: false,
        }
    }

    pub fn to_u32(&self) -> u32 {
        let mut val = 0;
        if self.acknowledged {
            val |= 1;
        }
        if self.driver {
            val |= 2;
        }
        if self.driver_ok {
            val |= 4;
        }
        if self.features_ok {
            val |= 8;
        }
        if self.failed {
            val |= 128;
        }
        val
    }
}

impl VirtioDevice {
    pub fn new(device_type: VirtioDeviceType) -> Self {
        Self {
            device_type,
            features: VirtioFeatures::empty(),
            status: VirtioStatus::new(),
            queue_num: 0,
            config: VirtioConfig {
                base_addr: 0,
                interrupt: 0,
            },
        }
    }

    pub fn probe(pci: &PciDevice) -> Option<Self> {
        println!("    Probing virtio device");
        Some(Self::new(VirtioDeviceType::Block))
    }

    pub fn initialize(&mut self) {
        self.status.acknowledged = true;
    }

    pub fn negotiate_features(&mut self, supported: u64) -> u64 {
        let negotiated = supported & self.features.bits();
        if negotiated != 0 {
            self.status.features_ok = true;
        }
        negotiated
    }

    pub fn set_driver_ok(&mut self) {
        self.status.driver_ok = true;
    }
}

pub trait VirtioDriver {
    fn device_type(&self) -> VirtioDeviceType;
    fn init(&mut self);
    fn handle_interrupt(&self);
}

pub struct VirtioBlkDriver {
    device: VirtioDevice,
    capacity: u64,
    block_size: u32,
    max_segments: u16,
}

impl VirtioBlkDriver {
    pub fn new() -> Self {
        Self {
            device: VirtioDevice::new(VirtioDeviceType::Block),
            capacity: 0,
            block_size: 512,
            max_segments: 0,
        }
    }

    pub fn read(&mut self, sector: u64, buffer: &mut [u8]) -> Result<(), VirtioError> {
        if buffer.len() < self.block_size as usize {
            return Err(VirtioError::InvalidParam);
        }

        let sector_bytes = sector * (self.block_size as u64);

        Ok(())
    }

    pub fn write(&mut self, sector: u64, data: &[u8]) -> Result<(), VirtioError> {
        if data.len() < self.block_size as usize {
            return Err(VirtioError::InvalidParam);
        }

        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), VirtioError> {
        Ok(())
    }
}

impl VirtioDriver for VirtioBlkDriver {
    fn device_type(&self) -> VirtioDeviceType {
        VirtioDeviceType::Block
    }

    fn init(&mut self) {
        self.device.initialize();
        self.capacity = 0;
    }

    fn handle_interrupt(&self) {}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VirtioError {
    InvalidParam,
    IoError,
    NotReady,
    NotSupported,
}
