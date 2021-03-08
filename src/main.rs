mod mos6502;
mod nes;
mod ram;
mod cartridge;
mod ppu;
mod mappers;

use std::{thread, time};
use std::cell::RefCell;
use std::rc::Rc;


fn main() {
    let bus = Rc::new(RefCell::new(nes::Nes::new()));
    let cartridge = Box::new(cartridge::Cartridge::new("Super_mario_brothers.nes"));
    let ram = Box::new(ram::Ram::new());
    let ppu = Box::new(ppu::Ppu::new(Rc::clone(&bus)));

    {
        let mut mut_bus = bus.borrow_mut();
        mut_bus.connect(cartridge);
        mut_bus.connect(ram);
        mut_bus.connect(ppu);
    }

    let mut cpu = mos6502::Cpu::new(Rc::clone(&bus));

    cpu.reset();

    loop {
        thread::sleep(time::Duration::from_millis(100));
        cpu.next_inst();
        println!("{:?}", cpu);
    }

   
    
}
