use log::error;

use crate::memory::device::Addressable;
use crate::video::DISPSTAT_ADDR;

use super::registers::{DispCnt, DispStat};
use super::{Frame, DISPCNT_ADDR, SCREEN_HEIGHT, SCREEN_WIDTH};

pub struct Ppu {
    pub h_counter: u16,
    pub scanline: u16,
    vram: Box<[u8; (0x07FFFFFF - 0x05000000) + 1]>,
    io: Box<[u8; (0x4000056 - 0x4000000) + 1]>,
    vblank_raised_for_frame: bool,
}

impl Ppu {
    pub fn new() -> Ppu {
        let vram = Box::<[u8; (0x07FFFFFF - 0x05000000) + 1]>::new_zeroed();
        let io = Box::<[u8; (0x4000056 - 0x4000000) + 1]>::new_zeroed();

        Ppu {
            h_counter: 0,
            scanline: 0,
            vram: unsafe { vram.assume_init() },
            io: unsafe { io.assume_init() },
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

            self.write_u16(
                DISPSTAT_ADDR,
                self.read_u16(DISPSTAT_ADDR) & DispStat::VBLANK_FLAG.bits(),
            );
        }

        if self.scanline >= 160 && !self.vblank_raised_for_frame {
            self.write_u16(
                DISPSTAT_ADDR,
                self.read_u16(DISPSTAT_ADDR) | DispStat::VBLANK_FLAG.bits(),
            );
            self.vblank_raised_for_frame = true;
        }
    }

    pub fn get_frame(&self) -> Frame {
        let lcd_control = self.read_as::<DispCnt>(DISPCNT_ADDR);
        match lcd_control.bg_mode() {
            3 => self.render_background_mode3(),
            4 => self.render_background_mode4(),
            mode => {
                error!("Unsupported PPU mode: {}", mode);
                [[(0, 0, 0); SCREEN_WIDTH]; SCREEN_HEIGHT]
            }
        }
    }

    fn render_background_mode3(&self) -> Frame {
        let mut frame = [[(0, 0, 0); SCREEN_WIDTH]; SCREEN_HEIGHT];

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let addr = 0x06000000 + ((y * SCREEN_WIDTH + x) as u32 * 2);
                let rgb = self.read_u16(addr);

                let (r, g, b) = (
                    ((rgb & 0b0000_0000_0001_1111) as u8),
                    (((rgb & 0b0000_0011_1110_0000) >> 5) as u8),
                    (((rgb & 0b0111_1100_0000_0000) >> 10) as u8),
                );

                frame[y][x] = (r << 3, g << 3, b << 3);
            }
        }

        frame
    }

    fn render_background_mode4(&self) -> Frame {
        let mut frame = [[(0, 0, 0); SCREEN_WIDTH]; SCREEN_HEIGHT];

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let addr = 0x06000000 + (y * SCREEN_WIDTH + x) as u32;
                let idx = self.read(addr) as u32;
                let rgb = self.read_u16(0x05000000 + (idx * 2));

                let (r, g, b) = (
                    ((rgb & 0b0000_0000_0001_1111) as u8),
                    (((rgb & 0b0000_0011_1110_0000) >> 5) as u8),
                    (((rgb & 0b0111_1100_0000_0000) >> 10) as u8),
                );

                frame[y][x] = (r << 3, g << 3, b << 3);
            }
        }

        frame
    }
}

impl Addressable for Ppu {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x04000000..=0x04000056 => self.io[(addr - 0x04000000) as usize],
            0x05000000..=0x07FFFFFF => self.vram[(addr - 0x05000000) as usize],
            _ => unreachable!(),
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0x04000000..=0x04000056 => self.io[(addr - 0x04000000) as usize] = value,
            0x05000000..=0x07FFFFFF => self.vram[(addr - 0x05000000) as usize] = value,
            _ => unreachable!(),
        }
    }
}
