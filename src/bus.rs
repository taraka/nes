pub trait BusDevice {
    fn read(&self, addr: u16) -> Option<u8>;
    fn write(&mut self, addr: u16, data: u8);
}


pub struct Bus {
    devices: Vec<Box<dyn BusDevice>>,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            devices: vec![],
        }
    }

    pub fn connect(&mut self, dev: Box<dyn BusDevice>) {
        self.devices.push(dev);
    }

    pub fn read(&self, addr: u16) -> u8 {
        self.devices.iter().find_map(|dev| dev.read(addr) ).unwrap_or(0)
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        for dev in self.devices.iter_mut() {
            dev.write(addr, data);
        }
    }
}