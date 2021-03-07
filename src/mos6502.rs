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

#[derive(Debug)]
enum AddrModeResult {
    Imp(),
    Abs(u16, u8),
    Rel(u16)
}

#[derive(Clone,Copy)]
struct Op <'a> {
    name: &'a str,
    op: fn(&mut Cpu<'a>, AddrModeResult) -> u8,
    addr_mode: fn(&mut Cpu<'a>) -> AddrModeResult,
    cycles: u8
}

pub struct Cpu<'a> {
    a: u8,
    x: u8,
    y: u8,
    pc: u16,
    sp: u8,
    status: u8,
    bus: &'a mut Bus<'a>,
    wait: u8,
    lookup: [Op<'a>; 256],
}

//Don't print the bus
use core::fmt::Debug;
impl Debug for Cpu <'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("Cpu")
            .field("a", &self.a)
            .field("x", &self.x)
            .field("y", &self.y)
            .field("pc", &self.pc)
            .field("sp", &self.sp)
            .field("wait", &self.wait)
            .finish()
    }
}

impl <'a> Cpu <'a> {
    pub fn new(bus: &'a mut Bus<'a>) -> Self {
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

    pub fn next_inst(&mut self) {
        while self.wait > 0 {
            self.clock();
        }

        self.clock();
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
        println!("{}", op.name);
        let amr = (op.addr_mode)(self);
        let additional_cycles = (op.op)(self, amr);
        self.wait = op.cycles + additional_cycles - 1;
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

    fn push(&mut self, value: u8) {
        self.bus.write(0x0100 + self.sp as u16, value);
        self.sp -= 1;
    }

    fn pop(&mut self) -> u8 {
        self.sp += 1;
        return self.bus.read(0x0100 + self.sp as u16);
    }

    fn push16(&mut self, value: u16) {
        self.push((value >> 8) as u8);
        self.push((value & 0x00ff) as u8);
    }

    fn pop16(&mut self) -> u16 {

        let lsb = self.pop() as u16;
        let msb = self.pop() as u16;

        return msb << 8 + lsb;
    }

    fn irq(&mut self) {
        if !self.get_flag(Flag::I) {
            self.push16(self.pc);

            self.set_flag(Flag::B, false);
            self.set_flag(Flag::U, true);
            self.set_flag(Flag::I, true);
            self.push(self.status);

            let lsb = self.bus.read(0xfffe) as u16;
            let msb = self.bus.read(0xffff) as u16;
            self.pc = (msb << 8) + lsb;
            self.wait = 7;
        }
    }

    fn nmi(&mut self) {
        self.push16(self.pc);

        self.set_flag(Flag::B, false);
        self.set_flag(Flag::U, true);
        self.set_flag(Flag::I, true);
        self.push(self.status);

        let lsb = self.bus.read(0xfffa) as u16;
        let msb = self.bus.read(0xfffb) as u16;

        self.pc = (msb << 8) + lsb;
        self.wait = 8;
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
    fn IMP(&mut self) -> AddrModeResult {
        AddrModeResult::Imp()
    }

    fn IMM(&mut self) -> AddrModeResult {
        self.pc +=1;
        AddrModeResult::Abs(self.pc, 0)
    }

    fn ZP0(&mut self) -> AddrModeResult {
        let addr = self.bus.read(self.pc);
        self.pc +=1;
        AddrModeResult::Abs(addr as u16, 0)
    }

    fn ZPX(&mut self) -> AddrModeResult {
        let addr = self.bus.read(self.pc + self.x as u16);
        self.pc +=1;
        AddrModeResult::Abs(addr as u16, 0)
    }

    fn ZPY(&mut self) -> AddrModeResult {
        let addr = self.bus.read(self.pc + self.y as u16);
        self.pc +=1;
        AddrModeResult::Abs(addr as u16, 0)
    }

    fn ABS(&mut self) -> AddrModeResult {
        let lsb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        let msb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        AddrModeResult::Abs((msb << 8) + lsb, 0)
    }

    fn ABX(&mut self) -> AddrModeResult {
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

        AddrModeResult::Abs(addr, c)
    }

    fn ABY(&mut self) -> AddrModeResult {
        let lsb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        let msb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        let addr = (msb << 8) + lsb + self.y as u16;

        // Have we changed page?
        let c = if (addr & 0xff00) != (msb<<8) {
            1
        } else {
            0
        };

        AddrModeResult::Abs(addr, c)
    }

    fn IND(&mut self) -> AddrModeResult {
        let ptr_lsb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        let ptr_msb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        let ptr = (ptr_msb << 8) + ptr_lsb;


        // Check for Bug
        if ptr_lsb == 0x00ff {
            AddrModeResult::Abs(((self.bus.read(ptr & 0xff00) as u16) << 8) + self.bus.read(ptr) as u16, 0)
        } 
        else {
            AddrModeResult::Abs(((self.bus.read(ptr+1)as u16) << 8) + self.bus.read(ptr) as u16, 0)
        }
    }

    fn IZX(&mut self) -> AddrModeResult {
        let addr = self.bus.read(self.pc) as u16;
        self.pc +=1;

        let lsb = self.bus.read(addr + self.x as u16) as u16;
        let msb = self.bus.read(addr + self.x as u16 + 1) as u16;

        AddrModeResult::Abs(msb << 8 + lsb, 0)
    }

    fn IZY(&mut self) -> AddrModeResult {
        let ptr = self.bus.read(self.pc) as u16;
        self.pc +=1;

        let lsb = self.bus.read(ptr) as u16;
        let msb = self.bus.read(ptr+ 1) as u16;

        let addr = (msb << 8) + lsb + self.y as u16;

        // Have we changed page?
        let c = if (addr & 0xff00) != (msb<<8) {
            1
        } else {
            0
        };

        AddrModeResult::Abs(addr, c)
    }

    fn REL(&mut self) -> AddrModeResult {
        let mut addr = self.bus.read(self.pc) as u16;
        self.pc +=1;

        if addr & 0x80 != 0 {
            addr |= 0xff00;
        }

        AddrModeResult::Rel(addr)
    }

    fn fetch(&mut self, amr: &AddrModeResult) -> u8 {
        match amr {
            AddrModeResult::Imp() => self.a,
            AddrModeResult::Abs(addr, _) => self.bus.read(*addr),
            AddrModeResult::Rel(addr) => 0 // TODO
        }
    }

    fn additional_cycles(&self, amr: &AddrModeResult, add: u8) -> u8 {
        match amr {
            AddrModeResult::Imp() => return 0,
            AddrModeResult::Abs(_, c) => c & add,
            AddrModeResult::Rel(_) => return 0
        }
    }

    // Operations
    fn AND(&mut self, amr: AddrModeResult) -> u8 {
        self.a = self.a & self.fetch(&amr);
        self.set_flag(Flag::Z, self.a == 0);
        self.set_flag(Flag::N, self.a & 80 != 0);
        return self.additional_cycles(&amr, 1);
    }

    fn ADC(&mut self, amr: AddrModeResult) -> u8 {
        let rhs = self.fetch(&amr);
        let (temp, overflow1) = self.a.overflowing_add(rhs);
        let (result, overflow2) = temp.overflowing_add(self.get_flag(Flag::C) as u8);

        self.set_flag(Flag::C, overflow1 || overflow2);
        self.set_flag(Flag::Z, result == 0);
        self.set_flag(Flag::N, result & 80 != 0);
        self.set_flag(Flag::V, (!(self.a ^ rhs) & (self.a ^ result)) & 0x80 != 0);

        self.a = result;
        
        return self.additional_cycles(&amr, 1);
    }

    fn ASL(&mut self, amr: AddrModeResult) -> u8 {
        let (temp, overflow) = self.fetch(&amr).overflowing_shl(1);

        self.set_flag(Flag::C, overflow);
        self.set_flag(Flag::Z, temp == 0);
        self.set_flag(Flag::N, temp & 80 != 0);

        match amr {
            AddrModeResult::Imp() => self.a = temp,
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, temp),
            _ => panic!("Invalid address mode"),
        }

        return self.additional_cycles(&amr, 0);
    }

    fn BCC(&mut self, amr: AddrModeResult) -> u8 {
        let mut cycles = 0;
        if !self.get_flag(Flag::C) {
            cycles += 1;

            let addr = match amr {
                AddrModeResult::Rel(addr_rel) => addr_rel + self.pc,
                _ => panic!("Branch must use Rel Addressing"),
            };
            
            if addr & 0xff00 != self.pc & 0xff00 {
                cycles += 1;
            }

            self.pc = addr;
        }

        return cycles;
    }

    fn BCS(&mut self, amr: AddrModeResult) -> u8 {
        let mut cycles = 0;
        if self.get_flag(Flag::C) {
            cycles += 1;

            let addr = match amr {
                AddrModeResult::Rel(addr_rel) => addr_rel + self.pc,
                _ => panic!("Branch must use Rel Addressing"),
            };
            
            if addr & 0xff00 != self.pc & 0xff00 {
                cycles += 1;
            }

            self.pc = addr;
        }

        return cycles;
    }

    fn BEQ(&mut self, amr: AddrModeResult) -> u8 {
        let mut cycles = 0;
        if self.get_flag(Flag::Z) {
            cycles += 1;

            let addr = match amr {
                AddrModeResult::Rel(addr_rel) => addr_rel + self.pc,
                _ => panic!("Branch must use Rel Addressing"),
            };
            
            if addr & 0xff00 != self.pc & 0xff00 {
                cycles += 1;
            }

            self.pc = addr;
        }

        return cycles;
    }

    fn BIT(&mut self, amr: AddrModeResult) -> u8 {
        let temp = self.fetch(&amr) & self.a;
        self.set_flag(Flag::Z, temp == 0x00);
        self.set_flag(Flag::N, temp  & (1<<7)!= 0x00);
        self.set_flag(Flag::V, temp  & (1<<6) != 0x00);
        return 0;
    }

    fn BMI(&mut self, amr: AddrModeResult) -> u8 {
        let mut cycles = 0;
        if self.get_flag(Flag::N) {
            cycles += 1;

            let addr = match amr {
                AddrModeResult::Rel(addr_rel) => addr_rel + self.pc,
                _ => panic!("Branch must use Rel Addressing"),
            };
            
            if addr & 0xff00 != self.pc & 0xff00 {
                cycles += 1;
            }

            self.pc = addr;
        }

        return cycles;
    }

    fn BNE(&mut self, amr: AddrModeResult) -> u8 {
        let mut cycles = 0;
        if !self.get_flag(Flag::Z) {
            cycles += 1;

            let addr = match amr {
                AddrModeResult::Rel(addr_rel) => addr_rel + self.pc,
                _ => panic!("Branch must use Rel Addressing"),
            };
            
            if addr & 0xff00 != self.pc & 0xff00 {
                cycles += 1;
            }

            self.pc = addr;
        }

        return cycles;
    }

    fn BPL(&mut self, amr: AddrModeResult) -> u8 {
        let mut cycles = 0;
        if !self.get_flag(Flag::N) {
            cycles += 1;

            let addr = match amr {
                AddrModeResult::Rel(addr_rel) => addr_rel + self.pc,
                _ => panic!("Branch must use Rel Addressing"),
            };
            
            if addr & 0xff00 != self.pc & 0xff00 {
                cycles += 1;
            }

            self.pc = addr;
        }

        return cycles;
    }

    fn BRK(&mut self, amr: AddrModeResult) -> u8 {
        self.pc += 1;

        self.set_flag(Flag::I, true);

        self.push16(self.pc);

        self.set_flag(Flag::B, true);
        
        self.push(self.status);

        self.set_flag(Flag::B, false);

        let lsb = self.bus.read(0xfffe) as u16;
        let msb = self.bus.read(0xffff) as u16;

        self.pc = (msb << 8) + lsb;
        return 0
    }

    fn BVC(&mut self, amr: AddrModeResult) -> u8 {
        let mut cycles = 0;
        if !self.get_flag(Flag::V) {
            cycles += 1;

            let addr = match amr {
                AddrModeResult::Rel(addr_rel) => addr_rel + self.pc,
                _ => panic!("Branch must use Rel Addressing"),
            };
            
            if addr & 0xff00 != self.pc & 0xff00 {
                cycles += 1;
            }

            self.pc = addr;
        }

        return cycles;
    }

    fn BVS(&mut self, amr: AddrModeResult) -> u8 {
        let mut cycles = 0;
        if self.get_flag(Flag::V) {
            cycles += 1;

            let addr = match amr {
                AddrModeResult::Rel(addr_rel) => addr_rel + self.pc,
                _ => panic!("Branch must use Rel Addressing"),
            };
            
            if addr & 0xff00 != self.pc & 0xff00 {
                cycles += 1;
            }

            self.pc = addr;
        }

        return cycles;
    }

    fn CLC(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::C, false);
        return 0;
    }

    fn CLD(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::D, false);
        return 0;
    }

    fn CLI(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::I, false);
        return 0;
    }

    fn CLV(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::V, false);
        return 0;
    }

    fn CMP(&mut self, amr: AddrModeResult) -> u8 {
        let fetched = self.fetch(&amr);
        let (result, overflow) = self.a.overflowing_sub(fetched);

        self.set_flag(Flag::C, self.a >= fetched);
        self.set_flag(Flag::Z, fetched == self.a);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        return 0;
    }

    fn CPX(&mut self, amr: AddrModeResult) -> u8 {
        let fetched = self.fetch(&amr);
        let (result, overflow) = self.x.overflowing_sub(fetched);

        self.set_flag(Flag::C, self.x >= fetched);
        self.set_flag(Flag::Z, fetched == self.x);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        return 0;
    }

    fn CPY(&mut self, amr: AddrModeResult) -> u8 {
        let fetched = self.fetch(&amr);
        let (result, overflow) = self.y.overflowing_sub(fetched);

        self.set_flag(Flag::C, self.x >= fetched);
        self.set_flag(Flag::Z, fetched == self.y);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        return 0;
    }

    fn DEC(&mut self, amr: AddrModeResult) -> u8 {

        let (result, overflow) = self.fetch(&amr).overflowing_sub(1);

        self.set_flag(Flag::Z, result == 0x00);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        self.x = result;
        return 0;
    }

    fn DEX(&mut self, amr: AddrModeResult) -> u8 {

        let (result, overflow) = self.x.overflowing_sub(1);

        self.set_flag(Flag::Z, result == 0x00);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        
        let addr = match amr {
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, result),
            _ => panic!("Branch must use Rel Addressing"),
        };
        
        return 0;
    }

    fn DEY(&mut self, amr: AddrModeResult) -> u8 {

        let (result, overflow) = self.y.overflowing_sub(1);

        self.set_flag(Flag::Z, result == 0x00);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        self.y = result;
        return 0;
    }

    fn EOR(&mut self, amr: AddrModeResult) -> u8 {
        self.a = self.fetch(&amr) ^ self.a;

        self.set_flag(Flag::Z, self.a == 0x00);
        self.set_flag(Flag::N, self.a & 0x80 != 0x00);

        return self.additional_cycles(&amr, 1);
    }

    fn INC(&mut self, amr: AddrModeResult) -> u8 {

        let (result, overflow) = self.fetch(&amr).overflowing_add(1);

        match amr {
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, result),
            _ => panic!("Invalid address mode"),
        }

        self.set_flag(Flag::Z, result == 0x00);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        return 0;
    }

    fn INX(&mut self, amr: AddrModeResult) -> u8 {

        let (result, overflow) = self.x.overflowing_add(1);

        self.set_flag(Flag::Z, result == 0x00);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        self.x = result;
        return 0;
    }

    fn INY(&mut self, amr: AddrModeResult) -> u8 {

        let (result, overflow) = self.y.overflowing_add(1);

        self.set_flag(Flag::Z, result == 0x00);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        self.y = result;
        return 0;
    }

    fn JMP(&mut self, amr: AddrModeResult) -> u8 {

        match amr {
            AddrModeResult::Abs(addr, _) => self.pc = addr,
            _ => panic!("Invalid address mode"),
        }

        return 0;
    }

    fn JSR(&mut self, amr: AddrModeResult) -> u8 {
        self.pc -= 1;
        self.push16(self.pc);

        match amr {
            AddrModeResult::Abs(addr, _) => self.pc = addr,
            _ => panic!("Invalid address mode"),
        }

        return 0;
    }

    fn LDA(&mut self, amr: AddrModeResult) -> u8 {
        self.a = self.fetch(&amr);
        println!("{:?} {:?}", amr, self.a);
        self.set_flag(Flag::Z, self.a == 0x00);
        self.set_flag(Flag::N, self.a & 0x80 != 0x00);
        return 0;
    }

    fn LDX(&mut self, amr: AddrModeResult) -> u8 {
        self.x = self.fetch(&amr);
        self.set_flag(Flag::Z, self.x == 0x00);
        self.set_flag(Flag::N, self.x & 0x80 != 0x00);
        return 0;
    }

    fn LDY(&mut self, amr: AddrModeResult) -> u8 {
        self.y = self.fetch(&amr);
        self.set_flag(Flag::Z, self.y == 0x00);
        self.set_flag(Flag::N, self.y & 0x80 != 0x00);
        return 0;
    }

    fn LSR(&mut self, amr: AddrModeResult) -> u8 {
        let fetched = self.fetch(&amr);
        self.set_flag(Flag::C, fetched == 0x01);

        let (temp, overflow) = fetched.overflowing_shr(1);

        self.set_flag(Flag::Z, temp == 0);
        self.set_flag(Flag::N, temp & 80 != 0);

        match amr {
            AddrModeResult::Imp() => self.a = temp,
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, temp),
            _ => panic!("Invalid address mode"),
        }

        return self.additional_cycles(&amr, 0);
    }

    fn NOP(&mut self, amr: AddrModeResult) -> u8 {
        //May need additional cycles for certain nops
        return self.additional_cycles(&amr, 0);
    }

    fn ORA(&mut self, amr: AddrModeResult) -> u8 {
        self.a = self.a | self.fetch(&amr);
        self.set_flag(Flag::Z, self.a == 0x00);
        self.set_flag(Flag::N, self.a & 0x80 != 0);
        return self.additional_cycles(&amr, 0);
    }

    fn PHA(&mut self, amr: AddrModeResult) -> u8 {
        self.push(self.a);
        return self.additional_cycles(&amr, 0);
    }

    fn PHP(&mut self, amr: AddrModeResult) -> u8 {
        self.push(self.status | Flag::B as u8 | Flag::U as u8);
        self.set_flag(Flag::B, false);
        self.set_flag(Flag::U, false);
        return self.additional_cycles(&amr, 0);
    }

    fn PLA(&mut self, amr: AddrModeResult) -> u8 {
        
        self.a = self.pop();
        self.set_flag(Flag::Z, self.a == 0x00);
        self.set_flag(Flag::N, self.a & 0x80 != 0);
        return self.additional_cycles(&amr, 0);
    }

    fn PLP(&mut self, amr: AddrModeResult) -> u8 {
        self.status = self.pop();
        self.set_flag(Flag::U, true);
        return self.additional_cycles(&amr, 0);
    }

    fn ROL(&mut self, amr: AddrModeResult) -> u8 {
        let fetched = self.fetch(&amr);
        let (temp, overflow) = fetched.overflowing_shl(1);
        let result = temp | self.get_flag(Flag::C) as u8;

        self.set_flag(Flag::C, overflow);
        self.set_flag(Flag::Z, result == 0);
        self.set_flag(Flag::N, result & 80 != 0);

        match amr {
            AddrModeResult::Imp() => self.a = result,
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, result),
            _ => panic!("Invalid address mode"),
        }

        return self.additional_cycles(&amr, 0);
    }

    fn ROR(&mut self, amr: AddrModeResult) -> u8 {
        let fetched = self.fetch(&amr);
        let (temp, overflow) = fetched.overflowing_shr(1);
        let result = temp | ((self.get_flag(Flag::C) as u8) << 7);

        self.set_flag(Flag::C, overflow);
        self.set_flag(Flag::Z, result == 0);
        self.set_flag(Flag::N, result & 80 != 0);

        match amr {
            AddrModeResult::Imp() => self.a = result,
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, result),
            _ => panic!("Invalid address mode"),
        }

        return self.additional_cycles(&amr, 0);
    }

    fn RTI(&mut self, amr: AddrModeResult) -> u8 {
        self.status = self.pop();

        self.status &= !(Flag::B as u8);
        self.status &= !(Flag::U as u8);

        self.pc = self.pop16();
        return self.additional_cycles(&amr, 0);
    }

    fn RTS(&mut self, amr: AddrModeResult) -> u8 {
        self.pc = self.pop16();
        return self.additional_cycles(&amr, 0);
    }

    fn SEC(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::C, true);
        return self.additional_cycles(&amr, 0);
    }

    fn SED(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::D, true);
        return self.additional_cycles(&amr, 0);
    }
    
    fn SEI(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::I, true);
        return self.additional_cycles(&amr, 0);
    }

    fn SBC(&mut self, amr: AddrModeResult) -> u8 {
        let rhs = self.fetch(&amr) ^ 0xff;
        let (temp, overflow1) = self.a.overflowing_add(rhs);
        let (result, overflow2) = temp.overflowing_add(self.get_flag(Flag::C) as u8);

        self.set_flag(Flag::C, overflow1 || overflow2);
        self.set_flag(Flag::Z, result == 0);
        self.set_flag(Flag::N, result & 80 != 0);
        self.set_flag(Flag::V, (!(self.a ^ rhs) & (self.a ^ result)) & 0x80 != 0);

        self.a = result;
        
        return self.additional_cycles(&amr, 1);
    }

    fn STA(&mut self, amr: AddrModeResult) -> u8 {
        match amr {
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, self.a),
            _ => panic!("Invalid address mode"),
        }
        return self.additional_cycles(&amr, 0);
    }

    fn STX(&mut self, amr: AddrModeResult) -> u8 {
        match amr {
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, self.x),
            _ => panic!("Invalid address mode"),
        }
        return self.additional_cycles(&amr, 0);
    }

    fn STY(&mut self, amr: AddrModeResult) -> u8 {
        match amr {
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, self.y),
            _ => panic!("Invalid address mode"),
        }
        return self.additional_cycles(&amr, 0);
    }

    fn TAX(&mut self, amr: AddrModeResult) -> u8 {
        self.x = self.a;
        self.set_flag(Flag::Z, self.x == 0x00);
        self.set_flag(Flag::N, self.x & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn TAY(&mut self, amr: AddrModeResult) -> u8 {
        self.y = self.a;
        self.set_flag(Flag::Z, self.y == 0x00);
        self.set_flag(Flag::N, self.y & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn TSX(&mut self, amr: AddrModeResult) -> u8 {
        self.x = self.sp;
        self.set_flag(Flag::Z, self.x == 0x00);
        self.set_flag(Flag::N, self.x & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn TXA(&mut self, amr: AddrModeResult) -> u8 {
        self.a = self.x;
        self.set_flag(Flag::Z, self.a == 0x00);
        self.set_flag(Flag::N, self.a & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn TXS(&mut self, amr: AddrModeResult) -> u8 {
        self.sp = self.x;
        return self.additional_cycles(&amr, 0);
    }

    fn TYA(&mut self, amr: AddrModeResult) -> u8 {
        self.a = self.y;
        self.set_flag(Flag::Z, self.a == 0x00);
        self.set_flag(Flag::N, self.a & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn XXX(&mut self, amr: AddrModeResult) -> u8 {
        self.additional_cycles(&amr, 0)
    }

    

    fn get_op_matrix() -> [Op<'a>; 256] {
        [
            Op{ name:"BRK", op: Self::BRK, addr_mode: Self::IMM, cycles: 7 },
            Op{ name:"ORA", op: Self::ORA, addr_mode: Self::IZX, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 3 },
            Op{ name:"ORA", op: Self::ORA, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"ASL", op: Self::ASL, addr_mode: Self::ZP0, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            Op{ name:"PHP", op: Self::PHP, addr_mode: Self::IMP, cycles: 3 },
            Op{ name:"ORA", op: Self::ORA, addr_mode: Self::IMM, cycles: 2 },
            Op{ name:"ASL", op: Self::ASL, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"ORA", op: Self::ORA, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"ASL", op: Self::ASL, addr_mode: Self::ABS, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"BPL", op: Self::BPL, addr_mode: Self::REL, cycles: 2 },
            Op{ name:"ORA", op: Self::ORA, addr_mode: Self::IZY, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"ORA", op: Self::ORA, addr_mode: Self::ZPX, cycles: 4 },
            Op{ name:"ASL", op: Self::ASL, addr_mode: Self::ZPX, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"CLC", op: Self::CLC, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"ORA", op: Self::ORA, addr_mode: Self::ABY, cycles: 4 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"ORA", op: Self::ORA, addr_mode: Self::ABX, cycles: 4 },
            Op{ name:"ASL", op: Self::ASL, addr_mode: Self::ABX, cycles: 7 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            Op{ name:"JSR", op: Self::JSR, addr_mode: Self::ABS, cycles: 6 },
            Op{ name:"AND", op: Self::AND, addr_mode: Self::IZX, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            Op{ name:"BIT", op: Self::BIT, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"AND", op: Self::AND, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"ROL", op: Self::ROL, addr_mode: Self::ZP0, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            Op{ name:"PLP", op: Self::PLP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"AND", op: Self::AND, addr_mode: Self::IMM, cycles: 2 },
            Op{ name:"ROL", op: Self::ROL, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"BIT", op: Self::BIT, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"AND", op: Self::AND, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"ROL", op: Self::ROL, addr_mode: Self::ABS, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"BMI", op: Self::BMI, addr_mode: Self::REL, cycles: 2 },
            Op{ name:"AND", op: Self::AND, addr_mode: Self::IZY, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"AND", op: Self::AND, addr_mode: Self::ZPX, cycles: 4 },
            Op{ name:"ROL", op: Self::ROL, addr_mode: Self::ZPX, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"SEC", op: Self::SEC, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"AND", op: Self::AND, addr_mode: Self::ABY, cycles: 4 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"AND", op: Self::AND, addr_mode: Self::ABX, cycles: 4 },
            Op{ name:"ROL", op: Self::ROL, addr_mode: Self::ABX, cycles: 7 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            Op{ name:"RTI", op: Self::RTI, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"EOR", op: Self::EOR, addr_mode: Self::IZX, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 3 },
            Op{ name:"EOR", op: Self::EOR, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"LSR", op: Self::LSR, addr_mode: Self::ZP0, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            Op{ name:"PHA", op: Self::PHA, addr_mode: Self::IMP, cycles: 3 },
            Op{ name:"EOR", op: Self::EOR, addr_mode: Self::IMM, cycles: 2 },
            Op{ name:"LSR", op: Self::LSR, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"JMP", op: Self::JMP, addr_mode: Self::ABS, cycles: 3 },
            Op{ name:"EOR", op: Self::EOR, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"LSR", op: Self::LSR, addr_mode: Self::ABS, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"BVC", op: Self::BVC, addr_mode: Self::REL, cycles: 2 },
            Op{ name:"EOR", op: Self::EOR, addr_mode: Self::IZY, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"EOR", op: Self::EOR, addr_mode: Self::ZPX, cycles: 4 },
            Op{ name:"LSR", op: Self::LSR, addr_mode: Self::ZPX, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"CLI", op: Self::CLI, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"EOR", op: Self::EOR, addr_mode: Self::ABY, cycles: 4 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"EOR", op: Self::EOR, addr_mode: Self::ABX, cycles: 4 },
            Op{ name:"LSR", op: Self::LSR, addr_mode: Self::ABX, cycles: 7 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            Op{ name:"RTS", op: Self::RTS, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"ADC", op: Self::ADC, addr_mode: Self::IZX, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 3 },
            Op{ name:"ADC", op: Self::ADC, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"ROR", op: Self::ROR, addr_mode: Self::ZP0, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            Op{ name:"PLA", op: Self::PLA, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"ADC", op: Self::ADC, addr_mode: Self::IMM, cycles: 2 },
            Op{ name:"ROR", op: Self::ROR, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"JMP", op: Self::JMP, addr_mode: Self::IND, cycles: 5 },
            Op{ name:"ADC", op: Self::ADC, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"ROR", op: Self::ROR, addr_mode: Self::ABS, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"BVS", op: Self::BVS, addr_mode: Self::REL, cycles: 2 },
            Op{ name:"ADC", op: Self::ADC, addr_mode: Self::IZY, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"ADC", op: Self::ADC, addr_mode: Self::ZPX, cycles: 4 },
            Op{ name:"ROR", op: Self::ROR, addr_mode: Self::ZPX, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"SEI", op: Self::SEI, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"ADC", op: Self::ADC, addr_mode: Self::ABY, cycles: 4 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"ADC", op: Self::ADC, addr_mode: Self::ABX, cycles: 4 },
            Op{ name:"ROR", op: Self::ROR, addr_mode: Self::ABX, cycles: 7 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"STA", op: Self::STA, addr_mode: Self::IZX, cycles: 6 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"STY", op: Self::STY, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"STA", op: Self::STA, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"STX", op: Self::STX, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 3 },
            Op{ name:"DEY", op: Self::DEY, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"TXA", op: Self::TXA, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"STY", op: Self::STY, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"STA", op: Self::STA, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"STX", op: Self::STX, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"BCC", op: Self::BCC, addr_mode: Self::REL, cycles: 2 },
            Op{ name:"STA", op: Self::STA, addr_mode: Self::IZY, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"STY", op: Self::STY, addr_mode: Self::ZPX, cycles: 4 },
            Op{ name:"STA", op: Self::STA, addr_mode: Self::ZPX, cycles: 4 },
            Op{ name:"STX", op: Self::STX, addr_mode: Self::ZPY, cycles: 4 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"TYA", op: Self::TYA, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"STA", op: Self::STA, addr_mode: Self::ABY, cycles: 5 },
            Op{ name:"TXS", op: Self::TXS, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 5 },
            Op{ name:"STA", op: Self::STA, addr_mode: Self::ABX, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            Op{ name:"LDY", op: Self::LDY, addr_mode: Self::IMM, cycles: 2 },
            Op{ name:"LDA", op: Self::LDA, addr_mode: Self::IZX, cycles: 6 },
            Op{ name:"LDX", op: Self::LDX, addr_mode: Self::IMM, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"LDY", op: Self::LDY, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"LDA", op: Self::LDA, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"LDX", op: Self::LDX, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 3 },
            Op{ name:"TAY", op: Self::TAY, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"LDA", op: Self::LDA, addr_mode: Self::IMM, cycles: 2 },
            Op{ name:"TAX", op: Self::TAX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"LDY", op: Self::LDY, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"LDA", op: Self::LDA, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"LDX", op: Self::LDX, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"BCS", op: Self::BCS, addr_mode: Self::REL, cycles: 2 },
            Op{ name:"LDA", op: Self::LDA, addr_mode: Self::IZY, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            Op{ name:"LDY", op: Self::LDY, addr_mode: Self::ZPX, cycles: 4 },
            Op{ name:"LDA", op: Self::LDA, addr_mode: Self::ZPX, cycles: 4 },
            Op{ name:"LDX", op: Self::LDX, addr_mode: Self::ZPY, cycles: 4 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"CLV", op: Self::CLV, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"LDA", op: Self::LDA, addr_mode: Self::ABY, cycles: 4 },
            Op{ name:"TSX", op: Self::TSX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"LDY", op: Self::LDY, addr_mode: Self::ABX, cycles: 4 },
            Op{ name:"LDA", op: Self::LDA, addr_mode: Self::ABX, cycles: 4 },
            Op{ name:"LDX", op: Self::LDX, addr_mode: Self::ABY, cycles: 4 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"CPY", op: Self::CPY, addr_mode: Self::IMM, cycles: 2 },
            Op{ name:"CMP", op: Self::CMP, addr_mode: Self::IZX, cycles: 6 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            Op{ name:"CPY", op: Self::CPY, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"CMP", op: Self::CMP, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"DEC", op: Self::DEC, addr_mode: Self::ZP0, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            Op{ name:"INY", op: Self::INY, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"CMP", op: Self::CMP, addr_mode: Self::IMM, cycles: 2 },
            Op{ name:"DEX", op: Self::DEX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"CPY", op: Self::CPY, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"CMP", op: Self::CMP, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"DEC", op: Self::DEC, addr_mode: Self::ABS, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"BNE", op: Self::BNE, addr_mode: Self::REL, cycles: 2 },
            Op{ name:"CMP", op: Self::CMP, addr_mode: Self::IZY, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"CMP", op: Self::CMP, addr_mode: Self::ZPX, cycles: 4 },
            Op{ name:"DEC", op: Self::DEC, addr_mode: Self::ZPX, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"CLD", op: Self::CLD, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"CMP", op: Self::CMP, addr_mode: Self::ABY, cycles: 4 },
            Op{ name:"NOP", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"CMP", op: Self::CMP, addr_mode: Self::ABX, cycles: 4 },
            Op{ name:"DEC", op: Self::DEC, addr_mode: Self::ABX, cycles: 7 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            Op{ name:"CPX", op: Self::CPX, addr_mode: Self::IMM, cycles: 2 },
            Op{ name:"SBC", op: Self::SBC, addr_mode: Self::IZX, cycles: 6 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            Op{ name:"CPX", op: Self::CPX, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"SBC", op: Self::SBC, addr_mode: Self::ZP0, cycles: 3 },
            Op{ name:"INC", op: Self::INC, addr_mode: Self::ZP0, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 5 },
            Op{ name:"INX", op: Self::INX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"SBC", op: Self::SBC, addr_mode: Self::IMM, cycles: 2 },
            Op{ name:"NOP", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::SBC, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"CPX", op: Self::CPX, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"SBC", op: Self::SBC, addr_mode: Self::ABS, cycles: 4 },
            Op{ name:"INC", op: Self::INC, addr_mode: Self::ABS, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"BEQ", op: Self::BEQ, addr_mode: Self::REL, cycles: 2 },
            Op{ name:"SBC", op: Self::SBC, addr_mode: Self::IZY, cycles: 5 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 8 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"SBC", op: Self::SBC, addr_mode: Self::ZPX, cycles: 4 },
            Op{ name:"INC", op: Self::INC, addr_mode: Self::ZPX, cycles: 6 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 6 },
            Op{ name:"SED", op: Self::SED, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"SBC", op: Self::SBC, addr_mode: Self::ABY, cycles: 4 },
            Op{ name:"NOP", op: Self::NOP, addr_mode: Self::IMP, cycles: 2 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
            Op{ name:"???", op: Self::NOP, addr_mode: Self::IMP, cycles: 4 },
            Op{ name:"SBC", op: Self::SBC, addr_mode: Self::ABX, cycles: 4 },
            Op{ name:"INC", op: Self::INC, addr_mode: Self::ABX, cycles: 7 },
            Op{ name:"???", op: Self::XXX, addr_mode: Self::IMP, cycles: 7 },
        ]
    } 
}