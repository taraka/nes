mod mos6502;
mod bus;
mod ram;


fn main() {
    let mut bus = bus::Bus::new();
    let ram = ram::Ram::new();

    bus.connect(&ram);

    let mut cpu = mos6502::Cpu::new(&bus);
    cpu.reset();

    println!("{:?}", cpu);
    
}
