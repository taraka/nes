use crate::bus::Bus;

enum Flag {
    C = (1 << 0), // Carry
    Z = (1 << 1), // Zero
    I = (1 << 2), //Disable irq
    D = (1 << 3), //Not used
    B = (1 << 4), // Break
    U = (1 << 5), // Ununsed
    V = (1 << 6), // Overflow
    N = (1 << 7), // Negative
}

enum AddrModeResult {
    Data(u8),
    Abs(u16),
    Rel(u16)
}

#[derive(Clone,Copy)]
struct Op <'a> {
    name: &'a str,
    op: fn(&mut Cpu<'a>, (AddrModeResult, u8)),
    addr_mode: fn(&mut Cpu<'a>) -> (AddrModeResult, u8),
    cycles: u8
}

pub struct Cpu<'a> {
    a: u8,
    x: u8,
    y: u8,
    pc: u16,
    sp: u8,
    status: u8,
    bus: &'a Bus<'a>,
    wait: u8,
    lookup: [Op<'a>; 1],
}

//Don't print the bus
use core::fmt::Debug;
impl Debug for Cpu <'_> {
    fn fmt(&self, _f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        Ok(())
    }
}

impl <'a> Cpu <'a> {
    pub fn new(bus: &'a Bus) -> Self {
        Self {
            a: 0,
            x: 0,
            y: 0,
            pc: 0,
            sp: 0xfd,
            status: 0,
            bus: bus,
            wait: 0,
            lookup: Self::get_op_matrix()
        }
    }

    pub fn clock(&mut self) {
        if self.wait > 0 {
            self.wait -= 1;
            return;
        }

        //Next instruction
        let opcode = self.bus.read(self.pc);
        self.pc += 1;

        let op = self.lookup[opcode as usize];
        let amr = (op.addr_mode)(self);
        (op.op)(self, amr);
        self.wait = op.cycles - 1;
    }

    pub fn reset(&mut self) {
        let lsb = self.bus.read(0xfffc) as u16;
        let msb = self.bus.read(0xfffd) as u16;

        self.pc = (msb << 8) + lsb;

        self.a = 0;
        self.x = 0;
        self.y = 0;


        self.sp = 0xfd;
        self.status = 0x00;
        self.set_flag(Flag::U, true);

        self.wait = 8;
    }

    fn irq() {

    }

    fn nmi() {
        
    }

    fn set_flag(&mut self, f: Flag, value: bool) {
        if value {
            self.status |= f as u8;
        }
        else {
            self.status &= !(f as u8);
        }
    }

    fn get_flag(&mut self, f: Flag) -> bool {
        self.status & f as u8 != 0
    }

    // Address modes
    fn IMP(&mut self) -> (AddrModeResult, u8) {
        return (AddrModeResult::Data(self.a), 0)
    }

    fn IMM(&mut self) -> (AddrModeResult, u8) {
        self.pc +=1;
        (AddrModeResult::Abs(self.pc), 0)
    }

    fn ZP0(&mut self) -> (AddrModeResult, u8) {
        let addr = self.bus.read(self.pc);
        self.pc +=1;
        (AddrModeResult::Abs(addr as u16), 0)
    }

    fn ZPX(&mut self) -> (AddrModeResult, u8) {
        let addr = self.bus.read(self.pc + self.x as u16);
        self.pc +=1;
        (AddrModeResult::Abs(addr as u16), 0)
    }

    fn ZPY(&mut self) -> (AddrModeResult, u8) {
        let addr = self.bus.read(self.pc + self.y as u16);
        self.pc +=1;
        (AddrModeResult::Abs(addr as u16), 0)
    }

    fn ABS(&mut self) -> (AddrModeResult, u8) {
        let lsb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        let msb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        (AddrModeResult::Abs((msb << 8) + lsb), 0)
    }

    fn ABX(&mut self) -> (AddrModeResult, u8) {
        let lsb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        let msb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        let addr = (msb << 8) + lsb + self.x as u16;

        // Have we changed page?
        let c = if (addr & 0xff00) != (msb<<8) {
            1
        } else {
            0
        };

        (AddrModeResult::Abs(addr), c)
    }

    fn IND(&mut self) -> (AddrModeResult, u8) {
        let ptr_lsb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        let ptr_msb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        let ptr = (ptr_msb << 8) + ptr_lsb;


        // Check for Bug
        if ptr_lsb == 0x00ff {
            (AddrModeResult::Abs(((self.bus.read(ptr & 0xff00) as u16) << 8) + self.bus.read(ptr) as u16), 0)
        } 
        else {
            (AddrModeResult::Abs(((self.bus.read(ptr+1)as u16) << 8) + self.bus.read(ptr) as u16), 0)
        }
    }

    fn IZX(&mut self) -> (AddrModeResult, u8) {
        let addr = self.bus.read(self.pc) as u16;
        self.pc +=1;

        let lsb = self.bus.read(addr + self.x as u16) as u16;
        let msb = self.bus.read(addr + self.x as u16 + 1) as u16;

        (AddrModeResult::Abs(msb << 8 + lsb), 0)
    }

    fn IZY(&mut self) -> (AddrModeResult, u8) {
        let ptr = self.bus.read(self.pc) as u16;
        self.pc +=1;

        let lsb = self.bus.read(ptr) as u16;
        let msb = self.bus.read(ptr+ 1) as u16;

        let addr = msb << 8 + lsb + self.y as u16;

        // Have we changed page?
        let c = if (addr & 0xff00) != (msb<<8) {
            1
        } else {
            0
        };

        (AddrModeResult::Abs(addr), c)
    }

    fn REL(&mut self) -> (AddrModeResult, u8) {
        let mut addr = self.bus.read(self.pc) as u16;
        self.pc +=1;

        if addr & 0x80 != 0 {
            addr |= 0xff00;
        }

        (AddrModeResult::Rel(addr), 0)
    }

    // Operations
    fn XXX(&mut self, amr: (AddrModeResult, u8)) {

    }

    fn BRK(&mut self, amr: (AddrModeResult, u8)) {

    }

    fn get_op_matrix() -> [Op<'a>; 1] {
        [
            Op{ name:"BRK", op: Self::BRK, addr_mode: Self::IMM, cycles: 7 },
            // Op{ name:"ORA", op: Self::ORA, addr_mode: Self::IZX, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 3 },
            // Op{ name:"ORA", op: Self::ORA, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"ASL", op: Self::ASL, addr_mode: Self::ZP0, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            // Op{ name:"PHP", op: Self::PHP, addr_mode: Self::IMP, cycles: 3 },
            // Op{ name:"ORA", op: Self::ORA, addr_mode: Self::IMM, cycles: 2 },
            // Op{ name:"ASL", op: Self::ASL, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"ORA", op: Self::ORA, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"ASL", op: Self::ASL, addr_mode: Self::ABS, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"BPL", op: Self::BPL, addr_mode: Self::REL, cycles: 2 },
            // Op{ name:"ORA", op: Self::ORA, addr_mode: Self::IZY, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"ORA", op: Self::ORA, addr_mode: Self::ZPX, cycles: 4 },
            // Op{ name:"ASL", op: Self::ASL, addr_mode: Self::ZPX, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"CLC", op: Self::CLC, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"ORA", op: Self::ORA, addr_mode: Self::ABY, cycles: 4 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"ORA", op: Self::ORA, addr_mode: Self::ABX, cycles: 4 },
            // Op{ name:"ASL", op: Self::ASL, addr_mode: Self::ABX, cycles: 7 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            // Op{ name:"JSR", op: Self::JSR, addr_mode: Self::ABS, cycles: 6 },
            // Op{ name:"AND", op: Self::AND, addr_mode: Self::IZX, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            // Op{ name:"BIT", op: Self::BIT, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"AND", op: Self::AND, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"ROL", op: Self::ROL, addr_mode: Self::ZP0, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            // Op{ name:"PLP", op: Self::PLP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"AND", op: Self::AND, addr_mode: Self::IMM, cycles: 2 },
            // Op{ name:"ROL", op: Self::ROL, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"BIT", op: Self::BIT, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"AND", op: Self::AND, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"ROL", op: Self::ROL, addr_mode: Self::ABS, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"BMI", op: Self::BMI, addr_mode: Self::REL, cycles: 2 },
            // Op{ name:"AND", op: Self::AND, addr_mode: Self::IZY, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"AND", op: Self::AND, addr_mode: Self::ZPX, cycles: 4 },
            // Op{ name:"ROL", op: Self::ROL, addr_mode: Self::ZPX, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"SEC", op: Self::SEC, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"AND", op: Self::AND, addr_mode: Self::ABY, cycles: 4 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"AND", op: Self::AND, addr_mode: Self::ABX, cycles: 4 },
            // Op{ name:"ROL", op: Self::ROL, addr_mode: Self::ABX, cycles: 7 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            // Op{ name:"RTI", op: Self::RTI, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"EOR", op: Self::EOR, addr_mode: Self::IZX, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 3 },
            // Op{ name:"EOR", op: Self::EOR, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"LSR", op: Self::LSR, addr_mode: Self::ZP0, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            // Op{ name:"PHA", op: Self::PHA, addr_mode: Self::IMP, cycles: 3 },
            // Op{ name:"EOR", op: Self::EOR, addr_mode: Self::IMM, cycles: 2 },
            // Op{ name:"LSR", op: Self::LSR, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"JMP", op: Self::JMP, addr_mode: Self::ABS, cycles: 3 },
            // Op{ name:"EOR", op: Self::EOR, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"LSR", op: Self::LSR, addr_mode: Self::ABS, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"BVC", op: Self::BVC, addr_mode: Self::REL, cycles: 2 },
            // Op{ name:"EOR", op: Self::EOR, addr_mode: Self::IZY, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"EOR", op: Self::EOR, addr_mode: Self::ZPX, cycles: 4 },
            // Op{ name:"LSR", op: Self::LSR, addr_mode: Self::ZPX, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"CLI", op: Self::CLI, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"EOR", op: Self::EOR, addr_mode: Self::ABY, cycles: 4 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"EOR", op: Self::EOR, addr_mode: Self::ABX, cycles: 4 },
            // Op{ name:"LSR", op: Self::LSR, addr_mode: Self::ABX, cycles: 7 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            // Op{ name:"RTS", op: Self::RTS, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"ADC", op: Self::ADC, addr_mode: Self::IZX, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 3 },
            // Op{ name:"ADC", op: Self::ADC, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"ROR", op: Self::ROR, addr_mode: Self::ZP0, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            // Op{ name:"PLA", op: Self::PLA, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"ADC", op: Self::ADC, addr_mode: Self::IMM, cycles: 2 },
            // Op{ name:"ROR", op: Self::ROR, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"JMP", op: Self::JMP, addr_mode: Self::IND, cycles: 5 },
            // Op{ name:"ADC", op: Self::ADC, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"ROR", op: Self::ROR, addr_mode: Self::ABS, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"BVS", op: Self::BVS, addr_mode: Self::REL, cycles: 2 },
            // Op{ name:"ADC", op: Self::ADC, addr_mode: Self::IZY, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"ADC", op: Self::ADC, addr_mode: Self::ZPX, cycles: 4 },
            // Op{ name:"ROR", op: Self::ROR, addr_mode: Self::ZPX, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"SEI", op: Self::SEI, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"ADC", op: Self::ADC, addr_mode: Self::ABY, cycles: 4 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"ADC", op: Self::ADC, addr_mode: Self::ABX, cycles: 4 },
            // Op{ name:"ROR", op: Self::ROR, addr_mode: Self::ABX, cycles: 7 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"STA", op: Self::STA, addr_mode: Self::IZX, cycles: 6 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"STY", op: Self::STY, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"STA", op: Self::STA, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"STX", op: Self::STX, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 3 },
            // Op{ name:"DEY", op: Self::DEY, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"TXA", op: Self::TXA, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"STY", op: Self::STY, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"STA", op: Self::STA, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"STX", op: Self::STX, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"BCC", op: Self::BCC, addr_mode: Self::REL, cycles: 2 },
            // Op{ name:"STA", op: Self::STA, addr_mode: Self::IZY, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"STY", op: Self::STY, addr_mode: Self::ZPX, cycles: 4 },
            // Op{ name:"STA", op: Self::STA, addr_mode: Self::ZPX, cycles: 4 },
            // Op{ name:"STX", op: Self::STX, addr_mode: Self::ZPY, cycles: 4 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"TYA", op: Self::TYA, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"STA", op: Self::STA, addr_mode: Self::ABY, cycles: 5 },
            // Op{ name:"TXS", op: Self::TXS, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 5 },
            // Op{ name:"STA", op: Self::STA, addr_mode: Self::ABX, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            // Op{ name:"LDY", op: Self::LDY, addr_mode: Self::IMM, cycles: 2 },
            // Op{ name:"LDA", op: Self::LDA, addr_mode: Self::IZX, cycles: 6 },
            // Op{ name:"LDX", op: Self::LDX, addr_mode: Self::IMM, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"LDY", op: Self::LDY, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"LDA", op: Self::LDA, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"LDX", op: Self::LDX, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 3 },
            // Op{ name:"TAY", op: Self::TAY, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"LDA", op: Self::LDA, addr_mode: Self::IMM, cycles: 2 },
            // Op{ name:"TAX", op: Self::TAX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"LDY", op: Self::LDY, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"LDA", op: Self::LDA, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"LDX", op: Self::LDX, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"BCS", op: Self::BCS, addr_mode: Self::REL, cycles: 2 },
            // Op{ name:"LDA", op: Self::LDA, addr_mode: Self::IZY, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            // Op{ name:"LDY", op: Self::LDY, addr_mode: Self::ZPX, cycles: 4 },
            // Op{ name:"LDA", op: Self::LDA, addr_mode: Self::ZPX, cycles: 4 },
            // Op{ name:"LDX", op: Self::LDX, addr_mode: Self::ZPY, cycles: 4 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"CLV", op: Self::CLV, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"LDA", op: Self::LDA, addr_mode: Self::ABY, cycles: 4 },
            // Op{ name:"TSX", op: Self::TSX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"LDY", op: Self::LDY, addr_mode: Self::ABX, cycles: 4 },
            // Op{ name:"LDA", op: Self::LDA, addr_mode: Self::ABX, cycles: 4 },
            // Op{ name:"LDX", op: Self::LDX, addr_mode: Self::ABY, cycles: 4 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"CPY", op: Self::CPY, addr_mode: Self::IMM, cycles: 2 },
            // Op{ name:"CMP", op: Self::CMP, addr_mode: Self::IZX, cycles: 6 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            // Op{ name:"CPY", op: Self::CPY, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"CMP", op: Self::CMP, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"DEC", op: Self::DEC, addr_mode: Self::ZP0, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            // Op{ name:"INY", op: Self::INY, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"CMP", op: Self::CMP, addr_mode: Self::IMM, cycles: 2 },
            // Op{ name:"DEX", op: Self::DEX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"CPY", op: Self::CPY, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"CMP", op: Self::CMP, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"DEC", op: Self::DEC, addr_mode: Self::ABS, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"BNE", op: Self::BNE, addr_mode: Self::REL, cycles: 2 },
            // Op{ name:"CMP", op: Self::CMP, addr_mode: Self::IZY, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"CMP", op: Self::CMP, addr_mode: Self::ZPX, cycles: 4 },
            // Op{ name:"DEC", op: Self::DEC, addr_mode: Self::ZPX, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"CLD", op: Self::CLD, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"CMP", op: Self::CMP, addr_mode: Self::ABY, cycles: 4 },
            // Op{ name:"NOP", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"CMP", op: Self::CMP, addr_mode: Self::ABX, cycles: 4 },
            // Op{ name:"DEC", op: Self::DEC, addr_mode: Self::ABX, cycles: 7 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            // Op{ name:"CPX", op: Self::CPX, addr_mode: Self::IMM, cycles: 2 },
            // Op{ name:"SBC", op: Self::SBC, addr_mode: Self::IZX, cycles: 6 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            // Op{ name:"CPX", op: Self::CPX, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"SBC", op: Self::SBC, addr_mode: Self::ZP0, cycles: 3 },
            // Op{ name:"INC", op: Self::INC, addr_mode: Self::ZP0, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            // Op{ name:"INX", op: Self::INX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"SBC", op: Self::SBC, addr_mode: Self::IMM, cycles: 2 },
            // Op{ name:"NOP", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::SBC, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"CPX", op: Self::CPX, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"SBC", op: Self::SBC, addr_mode: Self::ABS, cycles: 4 },
            // Op{ name:"INC", op: Self::INC, addr_mode: Self::ABS, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"BEQ", op: Self::BEQ, addr_mode: Self::REL, cycles: 2 },
            // Op{ name:"SBC", op: Self::SBC, addr_mode: Self::IZY, cycles: 5 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"SBC", op: Self::SBC, addr_mode: Self::ZPX, cycles: 4 },
            // Op{ name:"INC", op: Self::INC, addr_mode: Self::ZPX, cycles: 6 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            // Op{ name:"SED", op: Self::SED, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"SBC", op: Self::SBC, addr_mode: Self::ABY, cycles: 4 },
            // Op{ name:"NOP", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            // Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            // Op{ name:"SBC", op: Self::SBC, addr_mode: Self::ABX, cycles: 4 },
            // Op{ name:"INC", op: Self::INC, addr_mode: Self::ABX, cycles: 7 },
            // Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
        ]
    } 
}