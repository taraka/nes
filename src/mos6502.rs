use crate::bus::Bus;

#[derive(Debug)]
pub struct Cpu<'a> {
    a: u8,
    x: u8,
    y: u8,
    pc: u16,
    sp: u8,
    status: u8,
    bus: &'a Bus<'a>,
}

impl <'a> Cpu <'a> {
    pub fn new(bus: &'a Bus) -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: 0,
            status: 0,
            bus: bus
        }
    }

    pub fn clock(&mut self) {
            
    }

    pub fn reset(&mut self) {
        let lsb = self.bus.read(0xfffc) as u16;
        let msb = self.bus.read(0xfffd) as u16;

        self.pc = (msb << 8) + lsb;
    }

}