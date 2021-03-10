use std::rc::Rc;
use std::cell::RefCell;
use crate::ppu::Ppu;
use crate::mos6502::Cpu;
use crate::cartridge::Cartridge;
use crate::ram::Ram;

pub trait BusDevice {
    fn read(&mut self, addr: u16) -> Option<u8>;
    fn write(&mut self, addr: u16, data: u8);
}

pub struct Nes <'a> {
    bus: Rc<RefCell<Bus>>,
    cpu:  Rc<RefCell<Cpu<'a>>>,
    ppu:  Rc<RefCell<Ppu>>,
    cartridge: Option<Cartridge>
}


pub struct Bus {
    devices: Vec<Rc<RefCell<dyn BusDevice>>>,
}

impl Bus {

}

impl Nes <'_> {
    pub fn new() -> Self {
        let bus = Rc::new(RefCell::new(Bus::new()));
        let ppu = Rc::new(RefCell::new(Ppu::new(Rc::clone(&bus))));
        let ram = Rc::new(RefCell::new(Ram::new()));

        {
            let mut mut_bus = bus.borrow_mut();
            mut_bus.connect(ram.clone());
            mut_bus.connect(ppu.clone());
        }

        let cpu = Rc::new(RefCell::new(Cpu::new(Rc::clone(&bus))));

        Self {
            cpu: cpu,
            ppu: ppu,
            bus: bus,
            cartridge: None,
        }
    }

    pub fn insert(&mut self, cartridge: Cartridge) {
        self.bus.borrow_mut().connect(Rc::new(RefCell::new(cartridge)));
    }

    pub fn clock(&mut self) {
        self.cpu.borrow_mut().clock();
    }

    pub fn reset(&mut self) {
        self.cpu.borrow_mut().reset();
    }
}


impl Bus {
    pub fn new() -> Self {
        Self {
            devices: vec![],
        }
    }

    pub fn connect(&mut self, dev: Rc<RefCell<dyn BusDevice>>) {
        self.devices.push(dev);
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        self.devices.iter_mut().find_map(|dev| dev.borrow_mut().read(addr) ).unwrap_or(0)
    }

    pub fn write(&mut self, addr: u16, data: u8) {
        for dev in self.devices.iter_mut() {
            dev.borrow_mut().write(addr, data);
        }
    }

    fn ppu_read(&self, addr: u16) -> Option<u8> {
        None
    }

    fn ppu_write(&mut self, addr: u16, data: u8) {
        
    }
}