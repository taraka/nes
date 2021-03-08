mod mos6502;
mod bus;
mod ram;

use bus::BusDevice;

use std::{thread, time};


fn main() {
    let mut bus = bus::Bus::new();
    let mut ram = ram::Ram::new();

    // Reset Vector
    ram.write(0xfffd, 0x00);
    ram.write(0x00fe, 0x01);

    let program = [0xa5, 0xfe, 0x85, 0x00, 0xa5, 0xfe, 0x29, 0x03, 0x18, 0x69, 0x02, 0x85, 0x01, 0xa5, 0xfe, 0xa0, 0x00, 0x91, 0x00, 0x4c, 0x00, 0x06];
    for (i, o) in program.iter().enumerate() {
        ram.write((i as u16) + 0x0000, *o);
    }

    bus.connect(&mut ram);
    let mut cpu = mos6502::Cpu::new(&mut bus);
    cpu.reset();

    loop {
        thread::sleep(time::Duration::from_millis(100));
        cpu.next_inst();
        println!("{:?}", cpu);
    }

    // println!("{:?}", cpu);
    // cpu.next_inst();

    // println!("{:?}", cpu);
    // cpu.next_inst();

    // println!("{:?}", cpu);
    // cpu.next_inst();

    // println!("{:?}", cpu);
    // cpu.next_inst();




    // println!("{:?}", cpu);
    // ram.print_range(0x8000..0x8100);
    
}
