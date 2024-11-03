use super::device::Addressable;
use crate::audio::apu::Apu;
use crate::input::joypad::Joypad;
use crate::video::ppu::Ppu;
use spdlog::prelude::*;

pub struct Mmio {
    pub internal_memory: Box<[u8; 0x04FFFFFF + 1]>,
    pub external_memory: Box<[u8; (0x0FFFFFFF - 0x08000000) + 1]>,
    pub ppu: Ppu,
    pub joypad: Joypad,
    pub apu: Apu,
}

impl Mmio {
    pub fn new() -> Mmio {
        let internal_memory = Box::<[u8; 0x05000000]>::new_zeroed();
        let external_memory = Box::<[u8; 0x08000000]>::new_zeroed();

        Mmio {
            internal_memory: unsafe { internal_memory.assume_init() },
            external_memory: unsafe { external_memory.assume_init() },
            ppu: Ppu::new(),
            joypad: Joypad::new(),
            apu: Apu::new(),
        }
    }

    pub fn tick_components(&mut self) {
        self.ppu.tick();
    }

    pub fn read(&self, addr: u32) -> u8 {
        match addr {
            0x04000000..=0x04000056 => self.ppu.read(addr),    // PPU I/O
            0x04000080..=0x0400008E => self.apu.read(addr),    // APU I/O
            0x04000130..=0x04000133 => self.joypad.read(addr), // Joypad I/O
            0x04000000..=0x040003FE => {
                error!("Unmapped I/O read: {:08x}", addr);
                0
            }
            0x00000000..=0x04FFFFFF => self.internal_memory[addr as usize],
            0x05000000..=0x07FFFFFF => self.ppu.read(addr),
            0x08000000..=0x09FFFFFF => self.external_memory[(addr - 0x08000000) as usize],
            0x0A000000..=0x0BFFFFFF => self.external_memory[(addr - 0x0A000000) as usize], // Mirror of 0x08000000..=0x09FFFFFF
            0x0C000000..=0x0DFFFFFF => self.external_memory[(addr - 0x0C000000) as usize], // Mirror of 0x08000000..=0x09FFFFFF
            0x0E000000..=0x0FFFFFFF => self.external_memory[(addr - 0x0E000000) as usize], // Mostly Gamepak SRAM
            _ => {
                error!("Reading from unmapped memory address: {:08x}", addr);
                0
            }
        }
    }

    pub fn read_u16(&self, addr: u32) -> u16 {
        u16::from_le_bytes([self.read(addr), self.read(addr + 1)])
    }

    pub fn read_u32(&self, addr: u32) -> u32 {
        u32::from_le_bytes([
            self.read(addr),
            self.read(addr + 1),
            self.read(addr + 2),
            self.read(addr + 3),
        ])
    }

    pub fn write(&mut self, addr: u32, value: u8) {
        trace!("Writing {:02x} to {:08x}", value, addr);

        match addr {
            0x04000000..=0x04000056 => self.ppu.write(addr, value), // PPU I/O
            0x04000080..=0x0400008E => self.apu.write(addr, value), // APU I/O
            0x04000130..=0x04000133 => self.joypad.write(addr, value), // Joypad I/O
            0x04000000..=0x040003FE => error!("Unmapped I/O write: {:02x} to {:08x}", value, addr),
            0x00000000..=0x04FFFFFF => self.internal_memory[addr as usize] = value,
            0x05000000..=0x07FFFFFF => self.ppu.write(addr, value),
            0x08000000..=0x09FFFFFF => self.external_memory[(addr - 0x08000000) as usize] = value,
            0x0A000000..=0x0BFFFFFF => self.external_memory[(addr - 0x0A000000) as usize] = value, // Mirror of 0x08000000..=0x09FFFFFF
            0x0C000000..=0x0DFFFFFF => self.external_memory[(addr - 0x0C000000) as usize] = value, // Mirror of 0x08000000..=0x09FFFFFF
            0x0E000000..=0x0FFFFFFF => self.external_memory[(addr - 0x0E000000) as usize] = value, // Mostly Gamepak SRAM
            _ => {
                error!("Writing to unmapped memory address: {:08x}", addr);
            }
        }
    }

    pub fn write_u16(&mut self, addr: u32, value: u16) {
        let [a, b] = value.to_le_bytes();
        self.write(addr, a);
        self.write(addr + 1, b);
    }

    pub fn write_u32(&mut self, addr: u32, value: u32) {
        let [a, b, c, d] = value.to_le_bytes();
        self.write(addr, a);
        self.write(addr + 1, b);
        self.write(addr + 2, c);
        self.write(addr + 3, d);
    }

    pub fn load(&mut self, addr: u32, data: &[u8]) {
        let addr = addr as usize;
        match addr {
            0x00000000..=0x04FFFFFF => self.internal_memory[addr..addr + data.len()].copy_from_slice(data),
            0x05000000..=0x07FFFFFF => self.ppu.load(addr as u32, data),
            0x08000000..=0x0FFFFFFF => {
                self.external_memory[(addr - 0x08000000)..(addr - 0x08000000) + data.len()].copy_from_slice(data)
            }
            _ => panic!("Invalid memory address: {:08x}", addr),
        }
    }
}
