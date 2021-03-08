use crate::mappers::Mapper;

#[derive(Debug)]
pub struct Mapper0 {
    prg_rom_banks: u8,
    mask:u16,
}


impl Mapper0 {
    pub fn new(prg_rom_banks: u8) -> Self {
        Self {
            prg_rom_banks: prg_rom_banks,
            mask: if prg_rom_banks > 1 {
                0x7fff
            }
            else {
                0x3fff
            }
        }
    }
}

impl Mapper for Mapper0 {
    fn read(&mut self, addr: u16) -> Option<u16> {
        if addr >= 0x8000 && addr <= 0xFFFF {
            return Some(addr & self.mask)
        }
        None
    }
    fn write(&mut self, addr: u16, data: u8) -> Option<u16> {
        if addr >= 0x8000 && addr <= 0xFFFF {
            return Some(addr & self.mask)
        }
        None
    }

    fn ppu_read(&mut self, addr: u16) -> Option<u16> {
        Some(addr)
    }
    fn ppu_write(&mut self, addr: u16, data: u8) -> Option<u16> {
        Some(addr)
    }
}