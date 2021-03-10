use crate::nes::BusDevice;
use crate::nes::Bus;
use std::cell::RefCell;
use std::rc::Rc;

//#[derive(Debug)]
pub struct Ppu {
    memory: [u8; 0x8],
    bus:  Rc<RefCell<Bus>>,
    scanline: i16,
    cycle: u16,
    frame_complete: bool,
}

impl Ppu {
    pub fn new(bus: Rc<RefCell<Bus>>) -> Self {
        Self {
            memory: [0xff; 0x8],
            bus: bus,
            cycle: 0,
            scanline: -1,
            frame_complete: false,
        }
    }
}

impl BusDevice for Ppu {

    // Control = 0
    // Mask = 1
    // Status = 2
    // OMA Address = 3
    // OMA Data = 4
    // Scroll = 5
    // PPU Address = 6
    // PPU data = 7

    fn read(&mut self, addr: u16) -> Option<u8> {
        if (0x2000..0x4000).contains(&addr) {
            Some(self.memory[(addr as usize) & 0x7])
        }
        else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        if (0x2000..0x4000).contains(&addr) {
            self.memory[(addr as usize) & 0x7] = data;
        }
    }
}