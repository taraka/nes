use crate::bus::BusDevice;

#[derive(Debug)]
pub struct Ram {
    memory: [u8; 65536],
}

impl Ram {
    pub fn new() -> Self {
        let mut data = [0; 65536];

        data[0xfffd] = 0x80;

        Self {
            memory: data,
        }
    }
}

impl BusDevice for Ram {
    fn read(&self, addr: u16) -> Option<u8> {
        Some(self.memory[addr as usize])
    }

    fn write(&mut self, addr: u16, data: u8) {
        self.memory[addr as usize] = data;
    }
}