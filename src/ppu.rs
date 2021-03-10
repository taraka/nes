use crate::nes::BusDevice;
use crate::nes::Bus;
use std::cell::RefCell;
use std::rc::Rc;

//#[derive(Debug)]
pub struct Ppu {
    memory: [u8; 0x8],
    pal: [(u8, u8, u8); 64],
    image: [(u8, u8, u8); 256*240],
    bus:  Rc<RefCell<Bus>>,
    scanline: i32,
    cycle: i32,
    frame_complete: bool,
}

impl Ppu {
    pub fn new(bus: Rc<RefCell<Bus>>) -> Self {
        Self {
            memory: [0xff; 0x8],
            bus: bus,
            cycle: 0,
            scanline: -1, //TODO: Make this -1
            frame_complete: false,
            image: [(0, 0, 0); 256*240],
            pal: Self::get_pal()
        }
    }

    pub fn clock(&mut self) {
        self.set_pixel(0x30);

        self.cycle += 1;

        if self.cycle >= 341 {
            self.cycle = 0;
            self.scanline += 1;
            if self.scanline >= 261 {
                self.scanline = -1;
                self.frame_complete = true;
            }
        }
    }

    fn set_pixel(&mut self, colour: u8) {
        if self.scanline >= 0 && self.cycle > 0 {
            self.image[(self.cycle * 256 + self.scanline) as usize] = self.pal[colour as usize];
        }
    }

    fn get_pal() -> [(u8, u8, u8); 64] {
        return [
            (84, 84, 84),
	        (0, 30, 116),
	        (8, 16, 144),
	        (48, 0, 136),
	        (68, 0, 100),
	        (92, 0, 48),
	        (84, 4, 0),
	        (60, 24, 0),
	        (32, 42, 0),
	        (8, 58, 0),
	        (0, 64, 0),
	        (0, 60, 0),
	        (0, 50, 60),
	        (0, 0, 0),
	        (0, 0, 0),
	        (0, 0, 0),
	        (152, 150, 152),
	        (8, 76, 196),
	        (48, 50, 236),
	        (92, 30, 228),
	        (136, 20, 176),
	        (160, 20, 100),
	        (152, 34, 32),
	        (120, 60, 0),
	        (84, 90, 0),
	        (40, 114, 0),
	        (8, 124, 0),
	        (0, 118, 40),
	        (0, 102, 120),
	        (0, 0, 0),
	        (0, 0, 0),
	        (0, 0, 0),
	        (236, 238, 236),
	        (76, 154, 236),
	        (120, 124, 236),
	        (176, 98, 236),
	        (228, 84, 236),
	        (236, 88, 180),
	        (236, 106, 100),
	        (212, 136, 32),
	        (160, 170, 0),
	        (116, 196, 0),
	        (76, 208, 32),
	        (56, 204, 108),
	        (56, 180, 204),
	        (60, 60, 60),
	        (0, 0, 0),
	        (0, 0, 0),
	        (236, 238, 236),
	        (168, 204, 236),
	        (188, 188, 236),
	        (212, 178, 236),
	        (236, 174, 236),
	        (236, 174, 212),
	        (236, 180, 176),
	        (228, 196, 144),
	        (204, 210, 120),
	        (180, 222, 120),
	        (168, 226, 144),
	        (152, 226, 180),
	        (160, 214, 228),
	        (160, 162, 160),
	        (0, 0, 0),
	        (0, 0, 0),
        ]
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