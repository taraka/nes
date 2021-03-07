mod mos6502;
mod bus;
mod ram;

use bus::BusDevice;


fn main() {
    let mut bus = bus::Bus::new();
    let mut ram = ram::Ram::new();

    // Reset Vector
    ram.write(0xfffd, 0x80);


    bus.connect(&mut ram);
    let mut cpu = mos6502::Cpu::new(&mut bus);
    cpu.reset();

    println!("{:?}", cpu);
    ram.print_range(0x8000..0x8100);
    
}
