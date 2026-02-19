//! PCI Bus support

#[derive(Debug, Clone, Copy)]
pub struct PciBus;

impl PciBus {
    pub const fn new() -> Self {
        Self
    }

    pub fn read_device(&self, bus: u8, dev: u8, func: u8) -> Option<PciDevice> {
        let vendor = self.read_config(bus, dev, func, 0);
        if vendor == 0xffff || vendor == 0 {
            return None;
        }

        Some(PciDevice {
            bus,
            device: dev,
            function: func,
            vendor_id: vendor,
            device_id: self.read_config(bus, dev, func, 2),
            class_code: self.read_config(bus, dev, func, 9) >> 16,
            subclass: (self.read_config(bus, dev, func, 9) >> 8) as u8,
            prog_if: self.read_config(bus, dev, func, 9) as u8,
            header_type: (self.read_config(bus, dev, func, 14) >> 16) as u8,
        })
    }

    fn read_config(&self, _bus: u8, _dev: u8, _func: u8, _reg: u8) -> u32 {
        0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub header_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PciClass {
    Unclassified = 0x00,
    MassStorage = 0x01,
    Network = 0x02,
    Display = 0x03,
    Multimedia = 0x04,
    Memory = 0x05,
    Bridge = 0x06,
    SimpleComm = 0x07,
    BaseSystem = 0x08,
    InputDevice = 0x09,
    Docking = 0x0a,
    Processor = 0x0b,
    SerialBus = 0x0c,
    Wireless = 0x0d,
    IntelligentIO = 0x0e,
    SatelliteComm = 0x0f,
    Crypto = 0x10,
    SignalProc = 0x11,
    Other = 0xff,
}

impl From<u8> for PciClass {
    fn from(value: u8) -> Self {
        match value {
            0x00 => Self::Unclassified,
            0x01 => Self::MassStorage,
            0x02 => Self::Network,
            0x03 => Self::Display,
            0x04 => Self::Multimedia,
            0x05 => Self::Memory,
            0x06 => Self::Bridge,
            0x07 => Self::SimpleComm,
            0x08 => Self::BaseSystem,
            0x09 => Self::InputDevice,
            0x0a => Self::Docking,
            0x0b => Self::Processor,
            0x0c => Self::SerialBus,
            0x0d => Self::Wireless,
            0x0e => Self::IntelligentIO,
            0x0f => Self::SatelliteComm,
            0x10 => Self::Crypto,
            0x11 => Self::SignalProc,
            _ => Self::Other,
        }
    }
}
