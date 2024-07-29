use log::{error, trace, warn};

use crate::memory::device::{Addressable, IoDevice};

use super::{
    registers::{DispCnt, DispStat},
    Frame, SCREEN_HEIGHT, SCREEN_WIDTH,
};

pub struct Ppu {
    pub h_counter: u16,
    pub scanline: u16,
    vram: Box<[u8; (0x07FFFFFF - 0x05000000) + 1]>,
    lcd_status: DispStat,
    lcd_control: DispCnt,
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
            lcd_control: DispCnt::empty(),
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
        match self.lcd_control.bg_mode() {
            3 => self.render_background_mode3(),
            4 => self.render_background_mode4(),
            mode => {
                warn!("Unsupported PPU mode: {}", mode);
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

impl IoDevice for Ppu {
    fn read_io(&self, addr: u32) -> u16 {
        match addr {
            0x0 => self.lcd_control.bits(),
            0x4 => self.lcd_status.bits(),
            _ => {
                error!("Invalid PPU read: {:08x}", addr);
                0x6969
            }
        }
    }

    fn write_io(&mut self, addr: u32, value: u16) {
        match addr {
            0x0 => self.lcd_control = DispCnt::from_bits_truncate(value),
            0x4 => self.lcd_status = DispStat::from_bits_truncate(value),
            _ => error!("Invalid PPU write: {:08x} = {:04x}", addr, value),
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
}
