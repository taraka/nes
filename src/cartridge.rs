use crate::nes::BusDevice;
use std::fs::File;
use std::io::Read;
use std::cmp::min;
use crate::mappers::Mapper;
use crate::mappers::mapper0::Mapper0;

#[derive(Debug)]
pub struct Cartridge {
    header: Header,
    mapper: Box<dyn Mapper>,
    prg_mem: Vec<u8>,
    chr_mem: Vec<u8>,

}

#[derive(Debug)]
struct Header {
    name: String,
    prg_rom_chunks: u8,
    chr_rom_chunks: u8,
    mapper1: u8,
    mapper2: u8,
    prg_ram_size: u8,
    tv_system1: u8,
    tv_system2: u8,
    //Derived
    mapper_id: u8,
}

impl Header {
    fn new(file: &Vec<u8>) -> Self {
        println!("{:?}", file[0..4].to_vec());
        Self {
            name: String::from_utf8(file[0..4].to_vec()).unwrap(),
            prg_rom_chunks: file[4],
            chr_rom_chunks: file[5],
            mapper1: file[6],
            mapper2: file[7],
            prg_ram_size: file[8],
            tv_system1: file[9],
            tv_system2: file[10],
            mapper_id: ((file[7] >> 4) << 4) | (file[6] >> 4)
        }
    }
}

impl Cartridge {
    pub fn new(filename: &str) -> Self {
        // This is all for type 1 files
        let file = Self::read_file(filename);
        let header = Header::new(&file);
        println!("{:?}", header);
        let prg_size = header.prg_rom_chunks as usize * 0x4000;
        let chr_size = header.chr_rom_chunks as usize * 0x2000;
        
        let mut offset = 16; // Header + padding (training)
        let mut prg_mem = file[offset..min(offset+prg_size, file.len())].to_vec();
        prg_mem.resize(prg_size, 0);
        //println!("{:?}", prg_mem);
        offset += prg_size;
        let mut chr_mem = file[offset..min(offset+chr_size, file.len())].to_vec();
        chr_mem.resize(chr_size, 0);
 
        let me = Self {
            mapper: Self::get_mapper(&header),
            header: header,
            prg_mem: prg_mem,
            chr_mem: chr_mem,
        };

        me
    }

    fn read_file(filename: &str) -> Vec<u8> {
        let mut f = File::open(&filename).expect("no file found");
        let metadata = std::fs::metadata(&filename).expect("unable to read metadata");
        let mut buffer = vec![0; metadata.len() as usize];
        f.read(&mut buffer).expect("buffer overflow");
        return buffer;
    }

    fn get_mapper(header: &Header) -> Box<dyn Mapper> {
        println!("Using Mapper: {}", header.mapper_id);

        match header.mapper_id {
            0 => Box::new(Mapper0::new(header.prg_rom_chunks)),
            _ => panic!("Unknown mapper id: {}", header.mapper_id)
        }
    }
    
    fn ppu_read(&mut self, addr: u16) -> Option<u8> {
        if let Some(a) = self.mapper.read(addr) {
            //println!("Reading: {:#04x}, {}", a, self.prg_mem[a as usize]);
            return Some(self.chr_mem[a as usize]);
        }
        None
    }

    fn ppu_write(&mut self, addr: u16, data: u8) {
        if let Some(a) = self.mapper.write(addr, data) {
            //println!("Reading: {:#04x}, {}", a, self.prg_mem[a as usize]);
            //self.chr_mem[a as usize]);
        }
    }

}

impl BusDevice for Cartridge {
    fn read(&mut self, addr: u16) -> Option<u8> {

        if (0x8000..=0xFFFF).contains(&addr) {
            if let Some(a) = self.mapper.read(addr) {
                //println!("Reading: {:#04x}, {}", a, self.prg_mem[a as usize]);
                return Some(self.prg_mem[a as usize]);
            }
            None
        }
        else {
            None
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        if (0x8000..=0xFFFF).contains(&addr) {
            //self.memory[(addr as usize) & 0x7ff] = data;
        }
    }
}