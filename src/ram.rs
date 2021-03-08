use crate::nes::BusDevice;

#[derive(Debug)]
pub struct Ram {
    memory: [u8; 0x800],
}

impl Ram {
    pub fn new() -> Self {
        Self {
            memory: [0; 0x800],
        }
    }

    pub fn print_range(&self, r: std::ops::Range<u16>) {
        for i in r {
            if i % 0x10 == 0 {
                print!("{:#04x}:   ", i);
            }
            print!("{:#02x} ", self.read(i).unwrap());
            if (i+1) % 0x10 == 0 {
                print!("\n");
            }
        }
        print!("\n");
    }
}

impl BusDevice for Ram {
    fn read(&self, addr: u16) -> Option<u8> {
        if (0x0000..0x2000).contains(&addr) {
            Some(self.memory[(addr as usize) & 0x7ff])
        }
        else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        if (0x0000..0x2000).contains(&addr) {
            self.memory[(addr as usize) & 0x7ff] = data;
        }
    }
}