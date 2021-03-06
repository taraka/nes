pub trait BusDevice {
    fn read(&self, addr: u16) -> Option<u8>;
    fn write(&mut self, addr: u16, data: u8);
}


pub struct Bus <'a> {
    devices: Vec<&'a dyn BusDevice>,
}

//Don't print the bus
use core::fmt::Debug;
impl Debug for Bus <'_> {
    fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}

impl <'a> Bus <'a> {
    pub fn new() -> Self {
        Self {
            devices: vec![],
        }
    }

    pub fn connect(&mut self, dev: &'a dyn BusDevice) {
        self.devices.push(dev);
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.devices.iter().find_map(|dev| dev.read(addr)).unwrap_or(0)
    }
}