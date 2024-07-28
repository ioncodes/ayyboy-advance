use log::{trace, warn};

use crate::memory::device::{Addressable, IoDevice};

use super::{registers::DispStat, Frame, SCREEN_HEIGHT, SCREEN_WIDTH};

pub struct Ppu {
    pub h_counter: u16,
    pub scanline: u16,
    vram: Box<[u8; (0x07FFFFFF - 0x05000000) + 1]>,
    lcd_status: DispStat,
    vblank_raised_for_frame: bool,
}

impl Ppu {
    pub fn new() -> Ppu {
        let vram = Box::<[u8; (0x07FFFFFF - 0x05000000) + 1]>::new_zeroed();

        Ppu {
            h_counter: 0,
            scanline: 0,
            vram: unsafe { vram.assume_init() },
            lcd_status: DispStat::empty(),
            vblank_raised_for_frame: false,
        }
    }

    pub fn tick(&mut self) {
        self.h_counter += 1;

        if self.h_counter == 240 {
            self.h_counter = 0;
            self.scanline += 1;
        }

        if self.scanline == 228 {
            self.scanline = 0;
            self.vblank_raised_for_frame = false;
            self.lcd_status.remove(DispStat::VBLANK_FLAG);
        }

        if self.scanline >= 160 && !self.vblank_raised_for_frame {
            self.lcd_status.insert(DispStat::VBLANK_FLAG);
            self.vblank_raised_for_frame = true;
        }

        trace!("scanline={}, h_counter={}", self.scanline, self.h_counter);
    }

    pub fn get_frame(&self) -> Frame {
        // background mode 4

        let mut frame = [[(0, 0, 0); SCREEN_WIDTH]; SCREEN_HEIGHT];

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let pixel_address = 0x06000000 + (y * SCREEN_WIDTH + x) as u32;
                let pixel_index = self.read(pixel_address) as u32;
                let rgb = self.read_u16(0x05000000 + (2 * pixel_index));
                let (r, g, b) = (
                    ((rgb & 0b0000_0000_0001_1111) as u8),
                    (((rgb & 0b0000_0011_1110_0000) >> 5) as u8),
                    (((rgb & 0b0111_1100_0000_0000) >> 10) as u8),
                );
                frame[y][x] = (
                    (r << 3) | (r >> 2),
                    (g << 3) | (g >> 2),
                    (b << 3) | (b >> 2),
                );
            }
        }

        frame
    }
}

impl IoDevice for Ppu {
    fn read_io(&self, addr: u32) -> u16 {
        match addr {
            0x4 => self.lcd_status.bits(),
            _ => panic!("Invalid PPU read: {:08x}", addr),
        }
    }

    fn write_io(&mut self, addr: u32, value: u16) {
        match addr {
            0x4 => self.lcd_status = DispStat::from_bits_truncate(value),
            _ => panic!("Invalid PPU write: {:08x}", addr),
        }
    }
}

impl Addressable for Ppu {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x05000000..=0x07FFFFFF => self.vram[(addr - 0x05000000) as usize],
            _ => unreachable!(),
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        warn!("write to PPU VRAM: {:08x} = {:02x}", addr, value);
        match addr {
            0x05000000..=0x07FFFFFF => self.vram[(addr - 0x05000000) as usize] = value,
            _ => unreachable!(),
        }
    }

    fn load(&mut self, addr: u32, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            self.write(addr + i as u32, byte);
        }
    }
}
