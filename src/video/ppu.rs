use super::registers::{DispCnt, DispStat};
use super::{Frame, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::memory::device::{Addressable, IoRegister};
use spdlog::prelude::*;

#[derive(PartialEq)]
pub enum PpuEvent {
    VBlank,
    HBlank,
}

pub struct Ppu {
    pub h_counter: u16,
    pub vram: Box<[u8; (0x07FFFFFF - 0x05000000) + 1]>,
    io: Box<[u8; (0x4000056 - 0x4000000) + 1]>,
    vblank_raised_for_frame: bool,
    // I/O Registers
    pub scanline: IoRegister,
    pub disp_stat: IoRegister<DispStat>,
    pub disp_cnt: IoRegister<DispCnt>,
}

impl Ppu {
    pub fn new() -> Ppu {
        let vram = Box::<[u8; (0x07FFFFFF - 0x05000000) + 1]>::new_zeroed();
        let io = Box::<[u8; (0x4000056 - 0x4000000) + 1]>::new_zeroed();

        Ppu {
            h_counter: 0,
            vram: unsafe { vram.assume_init() },
            io: unsafe { io.assume_init() },
            vblank_raised_for_frame: false,
            scanline: IoRegister::default(),
            disp_stat: IoRegister::default(),
            disp_cnt: IoRegister::default(),
        }
    }

    pub fn tick(&mut self) -> Vec<PpuEvent> {
        let mut events = Vec::new();

        self.h_counter += 1;

        if self.h_counter == 0 {
            self.disp_stat.clear_flags(DispStat::HBLANK_FLAG);
        }

        if self.h_counter == 240 {
            self.h_counter = 0;
            self.scanline.0 += 1;
            events.push(PpuEvent::HBlank);
            self.disp_stat.set_flags(DispStat::HBLANK_FLAG);
        }

        if self.scanline.0 == 228 {
            self.scanline.0 = 0;
            self.vblank_raised_for_frame = false;
            self.disp_stat.clear_flags(DispStat::VBLANK_FLAG);
        }

        if self.scanline.0 >= 160 && !self.vblank_raised_for_frame {
            self.vblank_raised_for_frame = true;
            events.push(PpuEvent::VBlank);
            self.disp_stat.set_flags(DispStat::VBLANK_FLAG);
        }

        events
    }

    pub fn get_frame(&self) -> Frame {
        let lcd_control = self.disp_cnt.value();
        trace!("Grabbing internal frame buffer for PPU mode: {}", lcd_control.bg_mode());

        let parse_bg_layer_view = |layer: DispCnt| {
            if lcd_control.contains(layer) {
                "on"
            } else {
                "off"
            }
        };

        match lcd_control.bg_mode() {
            0..=2 => {
                trace!(
                    "Background layers: BG0({}), BG1({}), BG2({}), BG3({})",
                    parse_bg_layer_view(DispCnt::BG0_ON),
                    parse_bg_layer_view(DispCnt::BG1_ON),
                    parse_bg_layer_view(DispCnt::BG2_ON),
                    parse_bg_layer_view(DispCnt::BG3_ON)
                );
                [[(0, 0, 0); SCREEN_WIDTH]; SCREEN_HEIGHT]
            }
            3 => self.render_background_mode3(lcd_control.frame_address()),
            4 => self.render_background_mode4(lcd_control.frame_address()),
            _ => unreachable!(), // todo: there might be 5?
        }
    }

    fn render_background_mode3(&self, base_addr: u32) -> Frame {
        trace!("Rendering background mode 3 @ {:08x}", base_addr);

        let mut frame = [[(0, 0, 0); SCREEN_WIDTH]; SCREEN_HEIGHT];

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let addr = base_addr + ((y * SCREEN_WIDTH + x) as u32 * 2);
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

    fn render_background_mode4(&self, base_addr: u32) -> Frame {
        trace!("Rendering background mode 4 @ {:08x}", base_addr);

        let mut frame = [[(0, 0, 0); SCREEN_WIDTH]; SCREEN_HEIGHT];

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let addr = base_addr + (y * SCREEN_WIDTH + x) as u32;
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
            0x04000000..=0x04000001 => self.disp_cnt.read(addr),  // DISPCNT
            0x04000004..=0x04000005 => self.disp_stat.read(addr), // DISPSTAT
            0x04000006..=0x04000007 => self.scanline.read(addr),  // VCOUNT
            // rest of the registers
            0x04000000..=0x04000056 => self.io[(addr - 0x04000000) as usize],
            0x05000000..=0x07FFFFFF => self.vram[(addr - 0x05000000) as usize],
            _ => unreachable!(),
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0x04000000..=0x04000001 => self.disp_cnt.write(addr, value), // DISPCNT
            0x04000004..=0x04000005 => self.disp_stat.write(addr, value), // DISPSTAT
            0x04000006..=0x04000007 => self.scanline.write(addr, value), // VCOUNT
            // rest of the registers
            0x04000000..=0x04000056 => self.io[(addr - 0x04000000) as usize] = value,
            0x05000000..=0x07FFFFFF => {
                trace!("Writing to VRAM address: {:08x} with value: {:02x}", addr, value);
                self.vram[(addr - 0x05000000) as usize] = value
            }
            _ => unreachable!(),
        }
    }
}
