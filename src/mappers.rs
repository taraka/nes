pub mod mapper0;


pub trait Mapper: std::fmt::Debug {
    fn read(&mut self, addr: u16) -> Option<u16>;
    fn write(&mut self, addr: u16, data: u8) -> Option<u16>;

    fn ppu_read(&mut self, addr: u16) -> Option<u16>;
    fn ppu_write(&mut self, addr: u16, data: u8) -> Option<u16>;
}