use crate::bus::Bus;

enum Flag {
    C = (1 << 0), // Carry
    Z = (1 << 1), // Zero
    I = (1 << 2), // Disable irq
    D = (1 << 3), // Not used
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
        //println!("{}", op.name);
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
    fn imp(&mut self) -> AddrModeResult {
        AddrModeResult::Imp()
    }

    fn imm(&mut self) -> AddrModeResult {
        self.pc +=1;
        AddrModeResult::Abs(self.pc, 0)
    }

    fn zp0(&mut self) -> AddrModeResult {
        let addr = self.bus.read(self.pc);
        self.pc +=1;
        AddrModeResult::Abs(addr as u16, 0)
    }

    fn zpx(&mut self) -> AddrModeResult {
        let addr = self.bus.read(self.pc + self.x as u16);
        self.pc +=1;
        AddrModeResult::Abs(addr as u16, 0)
    }

    fn zpy(&mut self) -> AddrModeResult {
        let addr = self.bus.read(self.pc + self.y as u16);
        self.pc +=1;
        AddrModeResult::Abs(addr as u16, 0)
    }

    fn abs(&mut self) -> AddrModeResult {
        let lsb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        let msb = self.bus.read(self.pc) as u16;
        self.pc +=1;
        AddrModeResult::Abs((msb << 8) + lsb, 0)
    }

    fn abx(&mut self) -> AddrModeResult {
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

    fn aby(&mut self) -> AddrModeResult {
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

    fn ind(&mut self) -> AddrModeResult {
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

    fn izx(&mut self) -> AddrModeResult {
        let addr = self.bus.read(self.pc) as u16;
        self.pc +=1;

        let lsb = self.bus.read(addr + self.x as u16) as u16;
        let msb = self.bus.read(addr + self.x as u16 + 1) as u16;

        AddrModeResult::Abs(msb << 8 + lsb, 0)
    }

    fn izy(&mut self) -> AddrModeResult {
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

    fn rel(&mut self) -> AddrModeResult {
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
            AddrModeResult::Rel(_addr) => 0 // TODO
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
    fn and(&mut self, amr: AddrModeResult) -> u8 {
        self.a = self.a & self.fetch(&amr);
        self.set_flag(Flag::Z, self.a == 0);
        self.set_flag(Flag::N, self.a & 80 != 0);
        return self.additional_cycles(&amr, 1);
    }

    fn adc(&mut self, amr: AddrModeResult) -> u8 {
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

    fn asl(&mut self, amr: AddrModeResult) -> u8 {
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

    fn bcc(&mut self, amr: AddrModeResult) -> u8 {
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

    fn bcs(&mut self, amr: AddrModeResult) -> u8 {
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

    fn beq(&mut self, amr: AddrModeResult) -> u8 {
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

    fn bit(&mut self, amr: AddrModeResult) -> u8 {
        let temp = self.fetch(&amr) & self.a;
        self.set_flag(Flag::Z, temp == 0x00);
        self.set_flag(Flag::N, temp  & (1<<7)!= 0x00);
        self.set_flag(Flag::V, temp  & (1<<6) != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn bmi(&mut self, amr: AddrModeResult) -> u8 {
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

    fn bne(&mut self, amr: AddrModeResult) -> u8 {
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

    fn bpl(&mut self, amr: AddrModeResult) -> u8 {
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

    fn brk(&mut self, amr: AddrModeResult) -> u8 {
        self.pc += 1;

        self.set_flag(Flag::I, true);

        self.push16(self.pc);

        self.set_flag(Flag::B, true);
        
        self.push(self.status);

        self.set_flag(Flag::B, false);

        let lsb = self.bus.read(0xfffe) as u16;
        let msb = self.bus.read(0xffff) as u16;

        self.pc = (msb << 8) + lsb;
        return self.additional_cycles(&amr, 0);
    }

    fn bvc(&mut self, amr: AddrModeResult) -> u8 {
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

    fn bvs(&mut self, amr: AddrModeResult) -> u8 {
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

    fn clc(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::C, false);
        return self.additional_cycles(&amr, 0);
    }

    fn cld(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::D, false);
        return self.additional_cycles(&amr, 0);
    }

    fn cli(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::I, false);
        return self.additional_cycles(&amr, 0);
    }

    fn clv(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::V, false);
        return self.additional_cycles(&amr, 0);
    }

    fn cmp(&mut self, amr: AddrModeResult) -> u8 {
        let fetched = self.fetch(&amr);
        let (result, _overflow) = self.a.overflowing_sub(fetched);

        self.set_flag(Flag::C, self.a >= fetched);
        self.set_flag(Flag::Z, fetched == self.a);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn cpx(&mut self, amr: AddrModeResult) -> u8 {
        let fetched = self.fetch(&amr);
        let (result, _overflow) = self.x.overflowing_sub(fetched);

        self.set_flag(Flag::C, self.x >= fetched);
        self.set_flag(Flag::Z, fetched == self.x);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn cpy(&mut self, amr: AddrModeResult) -> u8 {
        let fetched = self.fetch(&amr);
        let (result, _overflow) = self.y.overflowing_sub(fetched);

        self.set_flag(Flag::C, self.x >= fetched);
        self.set_flag(Flag::Z, fetched == self.y);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn dec(&mut self, amr: AddrModeResult) -> u8 {

        let (result, _overflow) = self.fetch(&amr).overflowing_sub(1);

        self.set_flag(Flag::Z, result == 0x00);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        self.x = result;
        return self.additional_cycles(&amr, 0);
    }

    fn dex(&mut self, amr: AddrModeResult) -> u8 {

        let (result, _overflow) = self.x.overflowing_sub(1);

        self.set_flag(Flag::Z, result == 0x00);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        
        match amr {
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, result),
            _ => panic!("Branch must use Rel Addressing"),
        };
        
        return self.additional_cycles(&amr, 0);
    }

    fn dey(&mut self, amr: AddrModeResult) -> u8 {

        let (result, _overflow) = self.y.overflowing_sub(1);

        self.set_flag(Flag::Z, result == 0x00);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        self.y = result;
        return self.additional_cycles(&amr, 0);
    }

    fn eor(&mut self, amr: AddrModeResult) -> u8 {
        self.a = self.fetch(&amr) ^ self.a;

        self.set_flag(Flag::Z, self.a == 0x00);
        self.set_flag(Flag::N, self.a & 0x80 != 0x00);

        return self.additional_cycles(&amr, 1);
    }

    fn inc(&mut self, amr: AddrModeResult) -> u8 {

        let (result, _overflow) = self.fetch(&amr).overflowing_add(1);

        match amr {
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, result),
            _ => panic!("Invalid address mode"),
        }

        self.set_flag(Flag::Z, result == 0x00);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        return 0;
    }

    fn inx(&mut self, amr: AddrModeResult) -> u8 {

        let (result, _overflow) = self.x.overflowing_add(1);

        self.set_flag(Flag::Z, result == 0x00);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        self.x = result;
        return self.additional_cycles(&amr, 0);
    }

    fn iny(&mut self, amr: AddrModeResult) -> u8 {

        let (result, _overflow) = self.y.overflowing_add(1);

        self.set_flag(Flag::Z, result == 0x00);
        self.set_flag(Flag::N, result & 0x80 != 0x00);
        self.y = result;
        return self.additional_cycles(&amr, 0);
    }

    fn jmp(&mut self, amr: AddrModeResult) -> u8 {

        match amr {
            AddrModeResult::Abs(addr, _) => self.pc = addr,
            _ => panic!("Invalid address mode"),
        }

        return self.additional_cycles(&amr, 0);
    }

    fn jsr(&mut self, amr: AddrModeResult) -> u8 {
        self.pc -= 1;
        self.push16(self.pc);

        match amr {
            AddrModeResult::Abs(addr, _) => self.pc = addr,
            _ => panic!("Invalid address mode"),
        }

        return self.additional_cycles(&amr, 0);
    }

    fn lda(&mut self, amr: AddrModeResult) -> u8 {
        self.a = self.fetch(&amr);
        self.set_flag(Flag::Z, self.a == 0x00);
        self.set_flag(Flag::N, self.a & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn ldx(&mut self, amr: AddrModeResult) -> u8 {
        self.x = self.fetch(&amr);
        self.set_flag(Flag::Z, self.x == 0x00);
        self.set_flag(Flag::N, self.x & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn ldy(&mut self, amr: AddrModeResult) -> u8 {
        self.y = self.fetch(&amr);
        self.set_flag(Flag::Z, self.y == 0x00);
        self.set_flag(Flag::N, self.y & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn lsr(&mut self, amr: AddrModeResult) -> u8 {
        let fetched = self.fetch(&amr);
        self.set_flag(Flag::C, fetched == 0x01);

        let (temp, _overflow) = fetched.overflowing_shr(1);

        self.set_flag(Flag::Z, temp == 0);
        self.set_flag(Flag::N, temp & 80 != 0);

        match amr {
            AddrModeResult::Imp() => self.a = temp,
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, temp),
            _ => panic!("Invalid address mode"),
        }

        return self.additional_cycles(&amr, 0);
    }

    fn nop(&mut self, amr: AddrModeResult) -> u8 {
        //May need additional cycles for certain nops
        return self.additional_cycles(&amr, 0);
    }

    fn ora(&mut self, amr: AddrModeResult) -> u8 {
        self.a = self.a | self.fetch(&amr);
        self.set_flag(Flag::Z, self.a == 0x00);
        self.set_flag(Flag::N, self.a & 0x80 != 0);
        return self.additional_cycles(&amr, 0);
    }

    fn pha(&mut self, amr: AddrModeResult) -> u8 {
        self.push(self.a);
        return self.additional_cycles(&amr, 0);
    }

    fn php(&mut self, amr: AddrModeResult) -> u8 {
        self.push(self.status | Flag::B as u8 | Flag::U as u8);
        self.set_flag(Flag::B, false);
        self.set_flag(Flag::U, false);
        return self.additional_cycles(&amr, 0);
    }

    fn pla(&mut self, amr: AddrModeResult) -> u8 {
        
        self.a = self.pop();
        self.set_flag(Flag::Z, self.a == 0x00);
        self.set_flag(Flag::N, self.a & 0x80 != 0);
        return self.additional_cycles(&amr, 0);
    }

    fn plp(&mut self, amr: AddrModeResult) -> u8 {
        self.status = self.pop();
        self.set_flag(Flag::U, true);
        return self.additional_cycles(&amr, 0);
    }

    fn rol(&mut self, amr: AddrModeResult) -> u8 {
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

    fn ror(&mut self, amr: AddrModeResult) -> u8 {
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

    fn rti(&mut self, amr: AddrModeResult) -> u8 {
        self.status = self.pop();

        self.status &= !(Flag::B as u8);
        self.status &= !(Flag::U as u8);

        self.pc = self.pop16();
        return self.additional_cycles(&amr, 0);
    }

    fn rts(&mut self, amr: AddrModeResult) -> u8 {
        self.pc = self.pop16();
        return self.additional_cycles(&amr, 0);
    }

    fn sec(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::C, true);
        return self.additional_cycles(&amr, 0);
    }

    fn sed(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::D, true);
        return self.additional_cycles(&amr, 0);
    }
    
    fn sei(&mut self, amr: AddrModeResult) -> u8 {
        self.set_flag(Flag::I, true);
        return self.additional_cycles(&amr, 0);
    }

    fn sbc(&mut self, amr: AddrModeResult) -> u8 {
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

    fn sta(&mut self, amr: AddrModeResult) -> u8 {
        match amr {
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, self.a),
            _ => panic!("Invalid address mode"),
        }
        return self.additional_cycles(&amr, 0);
    }

    fn stx(&mut self, amr: AddrModeResult) -> u8 {
        match amr {
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, self.x),
            _ => panic!("Invalid address mode"),
        }
        return self.additional_cycles(&amr, 0);
    }

    fn sty(&mut self, amr: AddrModeResult) -> u8 {
        match amr {
            AddrModeResult::Abs(addr, _) => self.bus.write(addr, self.y),
            _ => panic!("Invalid address mode"),
        }
        return self.additional_cycles(&amr, 0);
    }

    fn tax(&mut self, amr: AddrModeResult) -> u8 {
        self.x = self.a;
        self.set_flag(Flag::Z, self.x == 0x00);
        self.set_flag(Flag::N, self.x & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn tay(&mut self, amr: AddrModeResult) -> u8 {
        self.y = self.a;
        self.set_flag(Flag::Z, self.y == 0x00);
        self.set_flag(Flag::N, self.y & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn tsx(&mut self, amr: AddrModeResult) -> u8 {
        self.x = self.sp;
        self.set_flag(Flag::Z, self.x == 0x00);
        self.set_flag(Flag::N, self.x & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn txa(&mut self, amr: AddrModeResult) -> u8 {
        self.a = self.x;
        self.set_flag(Flag::Z, self.a == 0x00);
        self.set_flag(Flag::N, self.a & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn txs(&mut self, amr: AddrModeResult) -> u8 {
        self.sp = self.x;
        return self.additional_cycles(&amr, 0);
    }

    fn tya(&mut self, amr: AddrModeResult) -> u8 {
        self.a = self.y;
        self.set_flag(Flag::Z, self.a == 0x00);
        self.set_flag(Flag::N, self.a & 0x80 != 0x00);
        return self.additional_cycles(&amr, 0);
    }

    fn xxx(&mut self, amr: AddrModeResult) -> u8 {
        self.additional_cycles(&amr, 0)
    }

    

    fn get_op_matrix() -> [Op<'a>; 256] {
        [
            Op{ name:"BRK", op: Self::brk, addr_mode: Self::imm, cycles: 7 },
            Op{ name:"ORA", op: Self::ora, addr_mode: Self::izx, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 8 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 3 },
            Op{ name:"ORA", op: Self::ora, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"ASL", op: Self::asl, addr_mode: Self::zp0, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 5 },
            Op{ name:"PHP", op: Self::php, addr_mode: Self::imp, cycles: 3 },
            Op{ name:"ORA", op: Self::ora, addr_mode: Self::imm, cycles: 2 },
            Op{ name:"ASL", op: Self::asl, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"ORA", op: Self::ora, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"ASL", op: Self::asl, addr_mode: Self::abs, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"BPL", op: Self::bpl, addr_mode: Self::rel, cycles: 2 },
            Op{ name:"ORA", op: Self::ora, addr_mode: Self::izy, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 8 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"ORA", op: Self::ora, addr_mode: Self::zpx, cycles: 4 },
            Op{ name:"ASL", op: Self::asl, addr_mode: Self::zpx, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"CLC", op: Self::clc, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"ORA", op: Self::ora, addr_mode: Self::aby, cycles: 4 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 7 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"ORA", op: Self::ora, addr_mode: Self::abx, cycles: 4 },
            Op{ name:"ASL", op: Self::asl, addr_mode: Self::abx, cycles: 7 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 7 },
            Op{ name:"JSR", op: Self::jsr, addr_mode: Self::abs, cycles: 6 },
            Op{ name:"AND", op: Self::and, addr_mode: Self::izx, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 8 },
            Op{ name:"BIT", op: Self::bit, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"AND", op: Self::and, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"ROL", op: Self::rol, addr_mode: Self::zp0, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 5 },
            Op{ name:"PLP", op: Self::plp, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"AND", op: Self::and, addr_mode: Self::imm, cycles: 2 },
            Op{ name:"ROL", op: Self::rol, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"BIT", op: Self::bit, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"AND", op: Self::and, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"ROL", op: Self::rol, addr_mode: Self::abs, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"BMI", op: Self::bmi, addr_mode: Self::rel, cycles: 2 },
            Op{ name:"AND", op: Self::and, addr_mode: Self::izy, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 8 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"AND", op: Self::and, addr_mode: Self::zpx, cycles: 4 },
            Op{ name:"ROL", op: Self::rol, addr_mode: Self::zpx, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"SEC", op: Self::sec, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"AND", op: Self::and, addr_mode: Self::aby, cycles: 4 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 7 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"AND", op: Self::and, addr_mode: Self::abx, cycles: 4 },
            Op{ name:"ROL", op: Self::rol, addr_mode: Self::abx, cycles: 7 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 7 },
            Op{ name:"RTI", op: Self::rti, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"EOR", op: Self::eor, addr_mode: Self::izx, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 8 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 3 },
            Op{ name:"EOR", op: Self::eor, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"LSR", op: Self::lsr, addr_mode: Self::zp0, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 5 },
            Op{ name:"PHA", op: Self::pha, addr_mode: Self::imp, cycles: 3 },
            Op{ name:"EOR", op: Self::eor, addr_mode: Self::imm, cycles: 2 },
            Op{ name:"LSR", op: Self::lsr, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"JMP", op: Self::jmp, addr_mode: Self::abs, cycles: 3 },
            Op{ name:"EOR", op: Self::eor, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"LSR", op: Self::lsr, addr_mode: Self::abs, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"BVC", op: Self::bvc, addr_mode: Self::rel, cycles: 2 },
            Op{ name:"EOR", op: Self::eor, addr_mode: Self::izy, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 8 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"EOR", op: Self::eor, addr_mode: Self::zpx, cycles: 4 },
            Op{ name:"LSR", op: Self::lsr, addr_mode: Self::zpx, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"CLI", op: Self::cli, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"EOR", op: Self::eor, addr_mode: Self::aby, cycles: 4 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 7 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"EOR", op: Self::eor, addr_mode: Self::abx, cycles: 4 },
            Op{ name:"LSR", op: Self::lsr, addr_mode: Self::abx, cycles: 7 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 7 },
            Op{ name:"RTS", op: Self::rts, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"ADC", op: Self::adc, addr_mode: Self::izx, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 8 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 3 },
            Op{ name:"ADC", op: Self::adc, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"ROR", op: Self::ror, addr_mode: Self::zp0, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 5 },
            Op{ name:"PLA", op: Self::pla, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"ADC", op: Self::adc, addr_mode: Self::imm, cycles: 2 },
            Op{ name:"ROR", op: Self::ror, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"JMP", op: Self::jmp, addr_mode: Self::ind, cycles: 5 },
            Op{ name:"ADC", op: Self::adc, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"ROR", op: Self::ror, addr_mode: Self::abs, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"BVS", op: Self::bvs, addr_mode: Self::rel, cycles: 2 },
            Op{ name:"ADC", op: Self::adc, addr_mode: Self::izy, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 8 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"ADC", op: Self::adc, addr_mode: Self::zpx, cycles: 4 },
            Op{ name:"ROR", op: Self::ror, addr_mode: Self::zpx, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"SEI", op: Self::sei, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"ADC", op: Self::adc, addr_mode: Self::aby, cycles: 4 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 7 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"ADC", op: Self::adc, addr_mode: Self::abx, cycles: 4 },
            Op{ name:"ROR", op: Self::ror, addr_mode: Self::abx, cycles: 7 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 7 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"STA", op: Self::sta, addr_mode: Self::izx, cycles: 6 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"STY", op: Self::sty, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"STA", op: Self::sta, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"STX", op: Self::stx, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 3 },
            Op{ name:"DEY", op: Self::dey, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"TXA", op: Self::txa, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"STY", op: Self::sty, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"STA", op: Self::sta, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"STX", op: Self::stx, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"BCC", op: Self::bcc, addr_mode: Self::rel, cycles: 2 },
            Op{ name:"STA", op: Self::sta, addr_mode: Self::izy, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"STY", op: Self::sty, addr_mode: Self::zpx, cycles: 4 },
            Op{ name:"STA", op: Self::sta, addr_mode: Self::zpx, cycles: 4 },
            Op{ name:"STX", op: Self::stx, addr_mode: Self::zpy, cycles: 4 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"TYA", op: Self::tya, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"STA", op: Self::sta, addr_mode: Self::aby, cycles: 5 },
            Op{ name:"TXS", op: Self::txs, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 5 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 5 },
            Op{ name:"STA", op: Self::sta, addr_mode: Self::abx, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 5 },
            Op{ name:"LDY", op: Self::ldy, addr_mode: Self::imm, cycles: 2 },
            Op{ name:"LDA", op: Self::lda, addr_mode: Self::izx, cycles: 6 },
            Op{ name:"LDX", op: Self::ldx, addr_mode: Self::imm, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"LDY", op: Self::ldy, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"LDA", op: Self::lda, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"LDX", op: Self::ldx, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 3 },
            Op{ name:"TAY", op: Self::tay, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"LDA", op: Self::lda, addr_mode: Self::imm, cycles: 2 },
            Op{ name:"TAX", op: Self::tax, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"LDY", op: Self::ldy, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"LDA", op: Self::lda, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"LDX", op: Self::ldx, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"BCS", op: Self::bcs, addr_mode: Self::rel, cycles: 2 },
            Op{ name:"LDA", op: Self::lda, addr_mode: Self::izy, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 5 },
            Op{ name:"LDY", op: Self::ldy, addr_mode: Self::zpx, cycles: 4 },
            Op{ name:"LDA", op: Self::lda, addr_mode: Self::zpx, cycles: 4 },
            Op{ name:"LDX", op: Self::ldx, addr_mode: Self::zpy, cycles: 4 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"CLV", op: Self::clv, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"LDA", op: Self::lda, addr_mode: Self::aby, cycles: 4 },
            Op{ name:"TSX", op: Self::tsx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"LDY", op: Self::ldy, addr_mode: Self::abx, cycles: 4 },
            Op{ name:"LDA", op: Self::lda, addr_mode: Self::abx, cycles: 4 },
            Op{ name:"LDX", op: Self::ldx, addr_mode: Self::aby, cycles: 4 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"CPY", op: Self::cpy, addr_mode: Self::imm, cycles: 2 },
            Op{ name:"CMP", op: Self::cmp, addr_mode: Self::izx, cycles: 6 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 8 },
            Op{ name:"CPY", op: Self::cpy, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"CMP", op: Self::cmp, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"DEC", op: Self::dec, addr_mode: Self::zp0, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 5 },
            Op{ name:"INY", op: Self::iny, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"CMP", op: Self::cmp, addr_mode: Self::imm, cycles: 2 },
            Op{ name:"DEX", op: Self::dex, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"CPY", op: Self::cpy, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"CMP", op: Self::cmp, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"DEC", op: Self::dec, addr_mode: Self::abs, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"BNE", op: Self::bne, addr_mode: Self::rel, cycles: 2 },
            Op{ name:"CMP", op: Self::cmp, addr_mode: Self::izy, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 8 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"CMP", op: Self::cmp, addr_mode: Self::zpx, cycles: 4 },
            Op{ name:"DEC", op: Self::dec, addr_mode: Self::zpx, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"CLD", op: Self::cld, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"CMP", op: Self::cmp, addr_mode: Self::aby, cycles: 4 },
            Op{ name:"NOP", op: Self::nop, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 7 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"CMP", op: Self::cmp, addr_mode: Self::abx, cycles: 4 },
            Op{ name:"DEC", op: Self::dec, addr_mode: Self::abx, cycles: 7 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 7 },
            Op{ name:"CPX", op: Self::cpx, addr_mode: Self::imm, cycles: 2 },
            Op{ name:"SBC", op: Self::sbc, addr_mode: Self::izx, cycles: 6 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 8 },
            Op{ name:"CPX", op: Self::cpx, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"SBC", op: Self::sbc, addr_mode: Self::zp0, cycles: 3 },
            Op{ name:"INC", op: Self::inc, addr_mode: Self::zp0, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 5 },
            Op{ name:"INX", op: Self::inx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"SBC", op: Self::sbc, addr_mode: Self::imm, cycles: 2 },
            Op{ name:"NOP", op: Self::nop, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::sbc, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"CPX", op: Self::cpx, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"SBC", op: Self::sbc, addr_mode: Self::abs, cycles: 4 },
            Op{ name:"INC", op: Self::inc, addr_mode: Self::abs, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"BEQ", op: Self::beq, addr_mode: Self::rel, cycles: 2 },
            Op{ name:"SBC", op: Self::sbc, addr_mode: Self::izy, cycles: 5 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 8 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"SBC", op: Self::sbc, addr_mode: Self::zpx, cycles: 4 },
            Op{ name:"INC", op: Self::inc, addr_mode: Self::zpx, cycles: 6 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 6 },
            Op{ name:"SED", op: Self::sed, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"SBC", op: Self::sbc, addr_mode: Self::aby, cycles: 4 },
            Op{ name:"NOP", op: Self::nop, addr_mode: Self::imp, cycles: 2 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 7 },
            Op{ name:"???", op: Self::nop, addr_mode: Self::imp, cycles: 4 },
            Op{ name:"SBC", op: Self::sbc, addr_mode: Self::abx, cycles: 4 },
            Op{ name:"INC", op: Self::inc, addr_mode: Self::abx, cycles: 7 },
            Op{ name:"???", op: Self::xxx, addr_mode: Self::imp, cycles: 7 },
        ]
    } 
}