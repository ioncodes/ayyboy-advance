use super::registers::{BgCnt, ColorDepth, DispCnt, DispStat};
use super::tile::Tile;
use super::{Frame, Rgb, PALETTE_ADDR_END, PALETTE_ADDR_START, PALETTE_TOTAL_ENTRIES, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::memory::device::{Addressable, IoRegister};
use crate::video::registers::InternalScreenSize;
use crate::video::tile::TileInfo;
use crate::video::{tile, TILEMAP_ENTRY_SIZE};
use log::*;

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
    pub bg_cnt: [IoRegister<BgCnt>; 4],
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
            bg_cnt: [IoRegister::default(); 4],
        }
    }

    pub fn tick(&mut self) -> Vec<PpuEvent> {
        let mut events = Vec::new();

        if self.h_counter == 0 {
            self.disp_stat.clear_flags(DispStat::HBLANK_FLAG);
        }

        self.h_counter += 1;

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
            0 => self.render_background_mode0(),
            1..=2 => {
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
            5 => self.render_background_mode5(lcd_control.frame_address()),
            _ => unreachable!(),
        }
    }

    pub fn get_background_frame(&self, mode: usize, base_addr: u32) -> Frame {
        match mode {
            0 => self.render_background_mode0(),
            1..=2 => [[(0, 0, 0); SCREEN_WIDTH]; SCREEN_HEIGHT],
            3 => self.render_background_mode3(base_addr),
            4 => self.render_background_mode4(base_addr),
            5 => self.render_background_mode5(base_addr),
            _ => unreachable!(),
        }
    }

    pub fn fetch_palette(&self) -> [Rgb; PALETTE_TOTAL_ENTRIES] {
        let mut palette = [(0u8, 0u8, 0u8); PALETTE_TOTAL_ENTRIES];

        for addr in (PALETTE_ADDR_START..=PALETTE_ADDR_END).step_by(2) {
            let rgb = self.read_u16(addr);
            let index = (addr - PALETTE_ADDR_START) as usize / 2;
            palette[index] = Self::extract_rgb(rgb);
        }

        palette
    }

    pub fn render_tileset(&self) -> Vec<Rgb> {
        let tileset_addr = self.bg_cnt[0].value().tileset_addr() as usize;
        let tile_size = match self.bg_cnt[0].value().bpp() {
            ColorDepth::Bpp4 => 0x20,
            ColorDepth::Bpp8 => 0x40,
        };
        let tile_count = match tile_size {
            0x20 => 1024,
            0x40 => 512,
            _ => unreachable!(),
        };
        let palettes = self.fetch_palette();
        let bank_size = if tile_size == 0x20 { 16 } else { 256 };
        let palette_bank0 = &palettes[0..bank_size];

        let mut tileset = vec![Tile::default(); tile_count]; // 64 pixels per tile

        for tile_id in 0..tile_count {
            let tile_addr = tileset_addr + (tile_id * tile_size);
            let tile_data = {
                let mut tile_data = vec![0u8; tile_size];
                for i in 0..tile_size {
                    tile_data[i] = self.read((tile_addr + i) as u32);
                }
                tile_data
            };

            let tile = Tile::from_bytes(&tile_data, palette_bank0);
            tileset[tile_id] = tile;
        }

        const TILE_WIDTH: usize = 8;
        const TILES_PER_ROW: usize = 16;
        let rows = tile_count / TILES_PER_ROW; // total rows
        let w_px = TILES_PER_ROW * TILE_WIDTH; // atlas width  in px (128)
        let h_px = rows * TILE_WIDTH; // atlas height in px (rows*8)

        let mut out = vec![(0, 0, 0); w_px * h_px];

        for (idx, tile) in tileset.iter().enumerate() {
            let gx = idx % TILES_PER_ROW; // tile X in grid
            let gy = idx / TILES_PER_ROW; // tile Y in grid
            let dst_x0 = gx * TILE_WIDTH;
            let dst_y0 = gy * TILE_WIDTH;

            for py in 0..TILE_WIDTH {
                for px in 0..TILE_WIDTH {
                    out[(dst_y0 + py) * w_px + dst_x0 + px] = tile.pixels[py * TILE_WIDTH + px];
                }
            }
        }

        out
    }

    fn render_background_mode0(&self) -> Frame {
        trace!("Rendering background mode");

        let palette = self.fetch_palette();

        let tileset_addr = self.bg_cnt[0].value().tileset_addr() as usize; // cbb
        let tilemap_addr = self.bg_cnt[0].value().tilemap_addr() as usize; // sbb

        let tile_size = match self.bg_cnt[0].value().bpp() {
            ColorDepth::Bpp4 => 0x20,
            ColorDepth::Bpp8 => 0x40,
        };
        let palette_bank_size = if tile_size == 0x20 { 16 } else { 256 };

        let (map_w, map_h, tiles_x, tiles_y) = match self.bg_cnt[0].value().screen_size() {
            InternalScreenSize::Size256x256 => (256, 256, 32, 32),
            InternalScreenSize::Size512x256 => (512, 256, 64, 32),
            InternalScreenSize::Size256x512 => (256, 512, 32, 64),
            InternalScreenSize::Size512x512 => (512, 512, 64, 64),
        };

        let mut frame = [[(0, 0, 0); SCREEN_WIDTH]; SCREEN_HEIGHT];
        let mut internal_frame = vec![(0, 0, 0); map_w * map_h];

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                let (block_col, block_row) = (tx / 32, ty / 32); // which 32×32 map
                let (local_col, local_row) = (tx & 31, ty & 31); // pos inside that map

                let block_index = match self.bg_cnt[0].value().screen_size() {
                    InternalScreenSize::Size256x256 => 0,
                    InternalScreenSize::Size512x256 => block_col,     // 0‥1
                    InternalScreenSize::Size256x512 => block_row * 2, // 0‥1 (stacked)
                    InternalScreenSize::Size512x512 => block_row * 2 + block_col, // 0‥3 (quad)
                };

                // fetch the tile from the tilemap
                let addr = tilemap_addr as u32
                    + (block_index * TILEMAP_ENTRY_SIZE) as u32
                    + (local_row * 32 + local_col) as u32 * 2; // 2 bytes per tile index
                let tile_info = TileInfo::from_bits_truncate(self.read_u16(addr));

                // fetch the tile data from the tileset
                let tile_addr = tileset_addr as usize + tile_info.tile_id() * 32; // 4-bpp ⇒ 32 B per tile
                let tile_data = {
                    let mut tile_data = vec![0u8; tile_size];
                    for i in 0..tile_size {
                        tile_data[i] = self.read((tile_addr + i) as u32);
                    }
                    tile_data
                };

                // extract the tile pixels using the given palette bank
                let palette_bank = palette
                    [tile_info.palette() * palette_bank_size..(tile_info.palette() + 1) * palette_bank_size]
                    .to_vec();
                let mut tile = Tile::from_bytes(&tile_data, &palette_bank);

                // flip the tile if needed
                if tile_info.contains(tile::TileInfo::FLIP_X) {
                    tile.flip_x();
                }

                if tile_info.contains(tile::TileInfo::FLIP_Y) {
                    tile.flip_y();
                }

                // render the tile to the internal frame buffer
                for y in 0..8 {
                    for x in 0..8 {
                        let pixel_x = tx * 8 + x;
                        let pixel_y = ty * 8 + y;

                        if pixel_x < map_w && pixel_y < map_h {
                            let pixel_color = tile.pixels[y * 8 + x];
                            internal_frame[pixel_y * map_w + pixel_x] = pixel_color;
                        }
                    }
                }
            }
        }

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                frame[y][x] = internal_frame[y * map_w + x];
            }
        }

        frame
    }

    fn render_background_mode3(&self, base_addr: u32) -> Frame {
        trace!("Rendering background mode 3 @ {:08x}", base_addr);

        let mut frame = [[(0, 0, 0); SCREEN_WIDTH]; SCREEN_HEIGHT];

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let addr = base_addr + ((y * SCREEN_WIDTH + x) as u32 * 2);
                let rgb = self.read_u16(addr);
                frame[y][x] = Self::extract_rgb(rgb);
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
                frame[y][x] = Self::extract_rgb(rgb);
            }
        }

        frame
    }

    fn render_background_mode5(&self, base_addr: u32) -> Frame {
        trace!("Rendering background mode 5 @ {:08x}", base_addr);

        let mut frame = [[(0, 0, 0); SCREEN_WIDTH]; SCREEN_HEIGHT];

        for y in 0..128 {
            for x in 0..160 {
                let addr = base_addr + ((y * SCREEN_WIDTH + x) as u32 * 2);
                let rgb = self.read_u16(addr);
                frame[y][x] = Self::extract_rgb(rgb);
            }
        }

        frame
    }

    fn extract_rgb(rgb: u16) -> (u8, u8, u8) {
        let r = ((rgb & 0b0000_0000_0001_1111) as u8) << 3;
        let g = (((rgb & 0b0000_0011_1110_0000) >> 5) as u8) << 3;
        let b = (((rgb & 0b0111_1100_0000_0000) >> 10) as u8) << 3;
        (r, g, b)
    }
}

impl Addressable for Ppu {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x04000000..=0x04000001 => self.disp_cnt.read(addr),  // DISPCNT
            0x04000004..=0x04000005 => self.disp_stat.read(addr), // DISPSTAT
            0x04000006..=0x04000007 => self.scanline.read(addr),  // VCOUNT
            0x04000008..=0x0400000E => {
                // BG0CNT, BG1CNT, BG2CNT, BG3CNT
                let index = (addr - 0x04000008) as usize / 2;
                self.bg_cnt[index].read(addr)
            }
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
            0x04000008..=0x0400000E => {
                // BG0CNT, BG1CNT, BG2CNT, BG3CNT
                let index = (addr - 0x04000008) as usize / 2;
                self.bg_cnt[index].write(addr, value)
            }
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
