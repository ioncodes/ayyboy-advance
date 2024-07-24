use crate::memory::device::IoDevice;

use super::registers::DispStat;

pub struct Ppu {
    pub h_counter: u16,
    pub scanline: u16,
    lcd_status: DispStat,
}

impl Ppu {
    pub fn new() -> Ppu {
        Ppu {
            h_counter: 0,
            scanline: 0,
            lcd_status: DispStat::empty(),
        }
    }

    pub fn tick(&mut self) {
        self.h_counter += 1;

        if self.h_counter == 240 {
            self.h_counter = 0;
            self.scanline += 1;
        }

        if self.scanline == 160 {
            self.lcd_status.insert(DispStat::VBLANK_FLAG);
        }

        if self.scanline == 228 {
            self.scanline = 0;
        }
    }
}

impl IoDevice for Ppu {
    fn read(&self, addr: u32) -> u16 {
        match addr {
            0x4 => self.lcd_status.bits(),
            _ => panic!("Invalid PPU read: {:08x}", addr),
        }
    }

    fn write(&mut self, addr: u32, value: u16) {
        match addr {
            0x4 => self.lcd_status = DispStat::from_bits_truncate(value),
            _ => panic!("Invalid PPU write: {:08x}", addr),
        }
    }
}
