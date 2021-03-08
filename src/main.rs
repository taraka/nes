mod mos6502;
mod nes;
mod ram;
mod cartridge;
mod ppu;

use nes::BusDevice;

use std::{thread, time};
use std::cell::RefCell;
use std::rc::Rc;


fn main() {
    let bus = Rc::new(RefCell::new(nes::Nes::new()));
    let cartridge = Box::new(cartridge::Cartridge::new());
    let mut ram = Box::new(ram::Ram::new());
    let ppu = Box::new(ppu::Ppu::new(Rc::clone(&bus)));

    // Fill Ram
    ram.write(0xfffd, 0x00);
    ram.write(0x00fe, 0x01);

    let program = [0xa5, 0xfe, 0x85, 0x00, 0xa5, 0xfe, 0x29, 0x03, 0x18, 0x69, 0x02, 0x85, 0x01, 0xa5, 0xfe, 0xa0, 0x00, 0x91, 0x00, 0x4c, 0x00, 0x06];
    for (i, o) in program.iter().enumerate() {
        ram.write((i as u16) + 0x0000, *o);
    }

    //println!("{:?}", cpu);
    ram.print_range(0x0000..0x0100);

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
