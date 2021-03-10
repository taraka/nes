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
    let mut nes = nes::Nes::new();
    let cartridge = cartridge::Cartridge::new("Super_mario_brothers.nes");

    nes.insert(cartridge);
    nes.reset();

    //println!("{:?}", cpu);
    loop {
        //thread::sleep(time::Duration::from_millis(10));
        //cpu.next_inst();
        //println!("{:?}", cpu);
        nes.clock();
    }

   
    
}
