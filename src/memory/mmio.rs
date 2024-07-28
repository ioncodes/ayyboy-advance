use log::{error, warn};

use crate::video::ppu::Ppu;

use super::device::{Addressable, IoDevice};

pub struct Mmio {
    pub internal_memory: Box<[u8; 0x04FFFFFF + 1]>,
    pub external_memory: Box<[u8; (0x0FFFFFFF - 0x08000000) + 1]>,
    pub ppu: Ppu,
}

impl Mmio {
    pub fn new() -> Mmio {
        let internal_memory = Box::<[u8; 0x05000000]>::new_zeroed();
        let external_memory = Box::<[u8; 0x08000000]>::new_zeroed();

        Mmio {
            internal_memory: unsafe { internal_memory.assume_init() },
            external_memory: unsafe { external_memory.assume_init() },
            ppu: Ppu::new(),
        }
    }

    pub fn tick_components(&mut self) {
        self.ppu.tick();
    }

    pub fn read(&self, addr: u32) -> u8 {
        match addr {
            0x04000000..=0x04000056 => panic!("8bit read from PPU register: {:08x}", addr),
            0x00000000..=0x04FFFFFF => self.internal_memory[addr as usize],
            0x05000000..=0x07FFFFFF => self.ppu.read(addr),
            0x08000000..=0x0FFFFFFF => self.external_memory[(addr - 0x08000000) as usize],
            _ => panic!("Invalid memory address: {:08x}", addr),
        }
    }

    pub fn read_u16(&self, addr: u32) -> u16 {
        u16::from_le_bytes([self.read(addr), self.read(addr + 1)])
    }

    pub fn read_u32(&self, addr: u32) -> u32 {
        // TODO: what if program tries to read a halfword from IO registers?
        match addr {
            0x04000000..=0x04000056 => return self.ppu.read_io(addr - 0x04000000) as u32,
            0x04000000..=0x040003FE => error!("16bit read from unmapped register: {:08x}", addr),
            _ => {}
        }

        u32::from_le_bytes([
            self.read(addr),
            self.read(addr + 1),
            self.read(addr + 2),
            self.read(addr + 3),
        ])
    }

    pub fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0x00000000..=0x04FFFFFF => self.internal_memory[addr as usize] = value,
            0x05000000..=0x07FFFFFF => self.ppu.write(addr, value),
            0x08000000..=0x0FFFFFFF => self.external_memory[(addr - 0x08000000) as usize] = value,
            _ => panic!("Invalid memory address: {:08x}", addr),
        }
    }

    pub fn write_u16(&mut self, addr: u32, value: u16) {
        let [a, b] = value.to_le_bytes();
        self.write(addr, a);
        self.write(addr + 1, b);
    }

    pub fn write_u32(&mut self, addr: u32, value: u32) {
        // TODO: what if program tries to write a halfword to IO registers?
        match addr {
            0x04000000..=0x04000056 => return self.ppu.write_io(addr - 0x04000000, value as u16),
            0x04000000..=0x040003FE => error!("16bit write to unmapped register: {:08x}", addr),
            _ => {}
        }

        let [a, b, c, d] = value.to_le_bytes();
        self.write(addr, a);
        self.write(addr + 1, b);
        self.write(addr + 2, c);
        self.write(addr + 3, d);
    }

    pub fn load(&mut self, addr: u32, data: &[u8]) {
        let addr = addr as usize;
        match addr {
            0x00000000..=0x04FFFFFF => {
                self.internal_memory[addr..addr + data.len()].copy_from_slice(data)
            }
            0x05000000..=0x07FFFFFF => self.ppu.load(addr as u32, data),
            0x08000000..=0x0FFFFFFF => self.external_memory
                [(addr - 0x08000000)..(addr - 0x08000000) + data.len()]
                .copy_from_slice(data),
            _ => panic!("Invalid memory address: {:08x}", addr),
        }
    }
}
