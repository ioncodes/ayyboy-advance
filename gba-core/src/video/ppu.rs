use super::registers::{BgCnt, BgOffset, ColorDepth, DispCnt, DispStat, ObjShape};
use super::tile::Tile;
use super::{Frame, Pixel, PALETTE_ADDR_END, PALETTE_ADDR_START, PALETTE_TOTAL_ENTRIES, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::memory::device::{Addressable, IoRegister};
use crate::video::registers::{Dimension, InternalScreenSize, ObjAttribute0, ObjAttribute1, ObjAttribute2, ObjSize};
use crate::video::tile::TileInfo;
use crate::video::TILEMAP_ENTRY_SIZE;
use log::*;

#[derive(PartialEq)]
pub enum PpuEvent {
    VBlank,
    HBlank,
}

#[derive(Clone)]
pub struct Sprite {
    pub id: usize,
    pub x: usize,
    pub y: usize,
    pub shape: ObjShape,
    pub size: ObjSize,
    pub tile_number: usize,
    pub palette: usize,
    pub x_flip: bool,
    pub y_flip: bool,
    pub priority: usize,
    pub image: Vec<Pixel>,
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
    pub bg_hofs: [IoRegister<BgOffset>; 4],
    pub bg_vofs: [IoRegister<BgOffset>; 4],
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
            bg_hofs: [IoRegister::default(); 4],
            bg_vofs: [IoRegister::default(); 4],
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

        let mut frame = match lcd_control.bg_mode() {
            0 => self.render_background_mode0(),
            1..=2 => {
                trace!(
                    "Background layers: BG0({}), BG1({}), BG2({}), BG3({})",
                    parse_bg_layer_view(DispCnt::BG0_ON),
                    parse_bg_layer_view(DispCnt::BG1_ON),
                    parse_bg_layer_view(DispCnt::BG2_ON),
                    parse_bg_layer_view(DispCnt::BG3_ON)
                );
                [[Pixel::Transparent; SCREEN_WIDTH]; SCREEN_HEIGHT]
            }
            3 => self.render_background_mode3(lcd_control.frame_address()),
            4 => self.render_background_mode4(lcd_control.frame_address()),
            5 => self.render_background_mode5(lcd_control.frame_address()),
            _ => unreachable!(),
        };

        self.render_sprites(&mut frame);

        frame
    }

    pub fn get_background_frame(&self, mode: usize, base_addr: u32) -> Frame {
        match mode {
            0 => self.render_background_mode0(),
            1..=2 => [[Pixel::Transparent; SCREEN_WIDTH]; SCREEN_HEIGHT],
            3 => self.render_background_mode3(base_addr),
            4 => self.render_background_mode4(base_addr),
            5 => self.render_background_mode5(base_addr),
            _ => unreachable!(),
        }
    }

    pub fn fetch_palette(&self) -> [Pixel; PALETTE_TOTAL_ENTRIES] {
        let mut palette = [Pixel::Transparent; PALETTE_TOTAL_ENTRIES];

        for addr in (PALETTE_ADDR_START..=PALETTE_ADDR_END).step_by(2) {
            let rgb = self.read_u16(addr);
            let index = (addr - PALETTE_ADDR_START) as usize / 2;
            palette[index] = Self::extract_rgb(rgb);
        }

        palette
    }

    pub fn render_tileset(&self) -> (usize, Vec<Pixel>) {
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

        let mut out = vec![palettes[0]; w_px * h_px];

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

        assert!(
            out.len() == w_px * h_px,
            "Tileset size mismatch: {} != {}",
            out.len(),
            w_px * h_px
        );

        (tile_count, out)
    }

    pub fn render_tilemap(&self, bg_cnt: &BgCnt) -> (InternalScreenSize, Vec<Pixel>) {
        let palette = self.fetch_palette();

        let tileset_addr = bg_cnt.tileset_addr() as usize; // cbb
        let tilemap_addr = bg_cnt.tilemap_addr() as usize; // sbb

        let tile_size = match bg_cnt.bpp() {
            ColorDepth::Bpp4 => 0x20,
            ColorDepth::Bpp8 => 0x40,
        };

        let (map_w, map_h, tiles_x, tiles_y) = match bg_cnt.screen_size() {
            InternalScreenSize::Size256x256 => (256, 256, 32, 32),
            InternalScreenSize::Size512x256 => (512, 256, 64, 32),
            InternalScreenSize::Size256x512 => (256, 512, 32, 64),
            InternalScreenSize::Size512x512 => (512, 512, 64, 64),
        };

        let mut internal_frame = vec![palette[0]; map_w * map_h];

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                let (block_col, block_row) = (tx / 32, ty / 32); // which 32×32 map
                let (local_col, local_row) = (tx & 31, ty & 31); // pos inside that map

                let block_index = match bg_cnt.screen_size() {
                    InternalScreenSize::Size256x256 => 0,
                    InternalScreenSize::Size512x256 => block_col, // 0‥1
                    InternalScreenSize::Size256x512 => block_row, // 0‥1 (stacked)
                    InternalScreenSize::Size512x512 => block_row * 2 + block_col, // 0‥3 (quad)
                };

                // fetch the tile from the tilemap
                let addr = tilemap_addr as u32
                    + (block_index * TILEMAP_ENTRY_SIZE) as u32
                    + (local_row * 32 + local_col) as u32 * 2; // 2 bytes per tile index
                let tile_info = TileInfo::from_bits_truncate(self.read_u16(addr));

                // fetch the tile data from the tileset
                let tile_addr = tileset_addr as usize + tile_info.tile_id() * tile_size;
                let tile_data = {
                    let mut tile_data = vec![0u8; tile_size];
                    for i in 0..tile_size {
                        tile_data[i] = self.read((tile_addr + i) as u32);
                    }
                    tile_data
                };

                // extract the tile pixels using the given palette bank
                let palette_bank = if tile_size == 0x20 {
                    &palette[tile_info.palette() as usize * 16..][..16]
                } else {
                    &palette[..256]
                };
                let mut tile = Tile::from_bytes(&tile_data, palette_bank);

                // flip the tile if needed
                if tile_info.contains(TileInfo::FLIP_X) {
                    tile.flip_x();
                }

                if tile_info.contains(TileInfo::FLIP_Y) {
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

        assert!(
            internal_frame.len() == map_w * map_h,
            "Internal frame size mismatch: {} != {}",
            internal_frame.len(),
            map_w * map_h
        );

        (bg_cnt.screen_size(), internal_frame)
    }

    pub fn create_sprite_debug_map(&self) -> Vec<Sprite> {
        const OAM_BASE: u32 = 0x0700_0000;
        const OBJ_BASE: u32 = 0x0601_0000;

        let mut sprites = Vec::with_capacity(128);

        let palette = self.fetch_palette();
        let obj_palette = &palette[256..512];

        let obj_dimension = self.disp_cnt.value().dimension();

        for obj_id in 0..128 {
            let attr0 = ObjAttribute0::from_bits_truncate(self.read_u16(OAM_BASE + obj_id * 8 + 0));
            let attr1 = ObjAttribute1::from_bits_truncate(self.read_u16(OAM_BASE + obj_id * 8 + 2));
            let attr2 = ObjAttribute2::from_bits_truncate(self.read_u16(OAM_BASE + obj_id * 8 + 4));

            let shape = attr0.shape();
            let size = attr1.size(shape);
            let (w_px, h_px) = Self::obj_dimensions(shape, size);

            let tile_size: usize = if attr0.bpp() == ColorDepth::Bpp8 { 0x40 } else { 0x20 };

            let tiles_per_row = if obj_dimension == Dimension::OneDimensional {
                w_px / 8
            } else {
                32
            };

            // tiles per dimension
            let tiles_x = w_px / 8;
            let tiles_y = h_px / 8;

            let mut sprite_data = vec![Pixel::Transparent; w_px * h_px];

            for ty in 0..tiles_y {
                for tx in 0..tiles_x {
                    let src_tx = if attr1.x_flip() { tiles_x - 1 - tx } else { tx };
                    let src_ty = if attr1.y_flip() { tiles_y - 1 - ty } else { ty };

                    let mut tile_nr = (attr2.tile_number() + src_ty * tiles_per_row + src_tx) as u32;
                    if attr0.bpp() == ColorDepth::Bpp8 {
                        // 1d 8bpp wrap
                        tile_nr &= 0x3FF;
                    }

                    // fetch raw tile bytes
                    let tile_addr = OBJ_BASE + (tile_nr * tile_size as u32);
                    let tile_data = {
                        let mut tile_data = vec![0u8; tile_size as usize]; // TODO: 64?
                        for i in 0..tile_size {
                            tile_data[i as usize] = self.read(tile_addr + i as u32);
                        }
                        tile_data
                    };

                    // extract the tile pixels using the given palette bank
                    let pal_slice = if attr0.bpp() == ColorDepth::Bpp4 {
                        &obj_palette[attr2.palette() as usize * 16..][..16]
                    } else {
                        &palette[256..512]
                    };
                    let mut tile = Tile::from_bytes(&tile_data[..tile_size], pal_slice);

                    // flip the tile if needed
                    if attr1.x_flip() {
                        tile.flip_x();
                    }
                    if attr1.y_flip() {
                        tile.flip_y();
                    }

                    for py in 0..8 {
                        for px in 0..8 {
                            let dst_x = tx * 8 + px;
                            let dst_y = ty * 8 + py;
                            sprite_data[dst_y * w_px + dst_x] = tile.pixels[py * 8 + px];
                        }
                    }
                }
            }

            sprites.push(Sprite {
                id: obj_id as usize,
                x: attr1.x_coordinate(),
                y: attr0.y_coordinate(),
                shape,
                size,
                tile_number: attr2.tile_number(),
                palette: attr2.palette(),
                x_flip: attr1.x_flip(),
                y_flip: attr1.y_flip(),
                priority: attr2.priority(),
                image: sprite_data,
            });
        }

        sprites
    }

    #[inline]
    fn obj_dimensions(shape: ObjShape, size: ObjSize) -> (usize, usize) {
        let dims = match size {
            ObjSize::Square8x8 => (8, 8),
            ObjSize::Square16x16 => (16, 16),
            ObjSize::Square32x32 => (32, 32),
            ObjSize::Square64x64 => (64, 64),
            ObjSize::Horizontal16x8 => (16, 8),
            ObjSize::Horizontal32x8 => (32, 8),
            ObjSize::Horizontal32x16 => (32, 16),
            ObjSize::Horizontal64x32 => (64, 32),
            ObjSize::Vertical8x16 => (8, 16),
            ObjSize::Vertical8x32 => (8, 32),
            ObjSize::Vertical16x32 => (16, 32),
            ObjSize::Vertical16x64 => (16, 64),
            ObjSize::Vertical32x64 => (32, 64),
        };

        assert!(
            match shape {
                ObjShape::Square => matches!(
                    size,
                    ObjSize::Square8x8 | ObjSize::Square16x16 | ObjSize::Square32x32 | ObjSize::Square64x64
                ),
                ObjShape::Horizontal => matches!(
                    size,
                    ObjSize::Horizontal16x8
                        | ObjSize::Horizontal32x8
                        | ObjSize::Horizontal32x16
                        | ObjSize::Horizontal64x32
                ),
                ObjShape::Vertical => matches!(
                    size,
                    ObjSize::Vertical8x16
                        | ObjSize::Vertical8x32
                        | ObjSize::Vertical16x32
                        | ObjSize::Vertical16x64
                        | ObjSize::Vertical32x64
                ),
            },
            "ObjShape({:?}) and ObjSize({:?}) mismatch",
            shape,
            size
        );

        dims
    }

    fn render_sprites(&self, frame: &mut [[Pixel; SCREEN_WIDTH]; SCREEN_HEIGHT]) {
        const OAM_BASE: u32 = 0x0700_0000;
        const OBJ_BASE: u32 = 0x0601_0000;

        let palette = self.fetch_palette();
        let obj_palette = &palette[256..512];

        let obj_dimension = self.disp_cnt.value().dimension();

        // lower OAM entry = higher priority
        // quick hack is to go through the OAM backwards
        for obj_id in (0..128).rev() {
            let attr0 = ObjAttribute0::from_bits_truncate(self.read_u16(OAM_BASE + obj_id * 8 + 0));
            let attr1 = ObjAttribute1::from_bits_truncate(self.read_u16(OAM_BASE + obj_id * 8 + 2));
            let attr2 = ObjAttribute2::from_bits_truncate(self.read_u16(OAM_BASE + obj_id * 8 + 4));

            // disabled, TODO: check if affine?
            if attr0.disabled() {
                continue;
            }

            let mut y = attr0.y_coordinate() as i32;
            if y >= 160 {
                y -= 256;
            }

            let mut x = attr1.x_coordinate() as i32;
            if x >= 240 {
                x -= 512;
            }

            let shape = attr0.shape();
            let size = attr1.size(shape);
            let (w_px, h_px) = Self::obj_dimensions(shape, size);

            // unsupported
            if w_px == 0 {
                continue;
            }

            let tile_size: usize = if attr0.bpp() == ColorDepth::Bpp8 { 0x40 } else { 0x20 };

            let tiles_per_row = if obj_dimension == Dimension::OneDimensional {
                w_px / 8
            } else {
                32
            };

            // tiles per dimension
            let tiles_x = w_px / 8;
            let tiles_y = h_px / 8;

            for ty in 0..tiles_y {
                for tx in 0..tiles_x {
                    let src_tx = if attr1.x_flip() { tiles_x - 1 - tx } else { tx };
                    let src_ty = if attr1.y_flip() { tiles_y - 1 - ty } else { ty };

                    let mut tile_nr = (attr2.tile_number() + src_ty * tiles_per_row + src_tx) as u32;
                    if attr0.bpp() == ColorDepth::Bpp8 {
                        // 1d 8bpp wrap
                        tile_nr &= 0x3FF;
                    }

                    // fetch raw tile bytes
                    let tile_addr = OBJ_BASE + (tile_nr * tile_size as u32);
                    let tile_data = {
                        let mut tile_data = vec![0u8; tile_size as usize]; // TODO: 64?
                        for i in 0..tile_size {
                            tile_data[i as usize] = self.read(tile_addr + i as u32);
                        }
                        tile_data
                    };

                    // extract the tile pixels using the given palette bank
                    let pal_slice = if attr0.bpp() == ColorDepth::Bpp4 {
                        &obj_palette[attr2.palette() as usize * 16..][..16]
                    } else {
                        &palette[256..512]
                    };
                    let mut tile = Tile::from_bytes(&tile_data[..tile_size], pal_slice);

                    // flip the tile if needed
                    if attr1.x_flip() {
                        tile.flip_x();
                    }
                    if attr1.y_flip() {
                        tile.flip_y();
                    }

                    // screen-space top-left of this 8x8 tile
                    let tile_x = x + (tx as i32) * 8;
                    let tile_y = y + (ty as i32) * 8;

                    // blit 8x8
                    for py in 0..8 {
                        let sy = tile_y + py as i32;
                        if sy < 0 || sy >= SCREEN_HEIGHT as i32 {
                            continue;
                        }

                        for px in 0..8 {
                            let sx = tile_x + px as i32;
                            if sx < 0 || sx >= SCREEN_WIDTH as i32 {
                                continue;
                            }

                            let c = tile.pixels[py * 8 + px];
                            // Instead of checking (0,0,0) for transparency:
                            if c != Pixel::Transparent {
                                frame[sy as usize][sx as usize] = c;
                            }
                        }
                    }
                }
            }
        }
    }

    fn render_background_mode0(&self) -> Frame {
        trace!("Rendering background mode 0");

        let mut frame = [[Pixel::Transparent; SCREEN_WIDTH]; SCREEN_HEIGHT];

        // this list is sorted by priority
        let bg_cnts = self.effective_backgrounds();

        for (id, bg_cnt) in bg_cnts {
            let (map_w, map_h) = match bg_cnt.screen_size() {
                InternalScreenSize::Size256x256 => (256, 256),
                InternalScreenSize::Size512x256 => (512, 256),
                InternalScreenSize::Size256x512 => (256, 512),
                InternalScreenSize::Size512x512 => (512, 512),
            };

            let vertical_offset = self.bg_vofs[id].value().offset();
            let horizontal_offset = self.bg_hofs[id].value().offset();

            let hoff = horizontal_offset % map_w;
            let voff = vertical_offset % map_h;

            let (_, tilemap) = self.render_tilemap(&bg_cnt);

            for y in 0..SCREEN_HEIGHT {
                let src_y = (y + voff) % map_h;
                for x in 0..SCREEN_WIDTH {
                    let src_x = (x + hoff) % map_w;
                    let color = tilemap[src_y * map_w + src_x];
                    // Instead of checking (0,0,0), we rely on whether color is Transparent.
                    if color != Pixel::Transparent {
                        frame[y][x] = color;
                    }
                }
            }
        }

        frame
    }

    fn render_background_mode3(&self, base_addr: u32) -> Frame {
        trace!("Rendering background mode 3 @ {:08x}", base_addr);

        let mut frame = [[Pixel::Transparent; SCREEN_WIDTH]; SCREEN_HEIGHT];

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

        let mut frame = [[Pixel::Transparent; SCREEN_WIDTH]; SCREEN_HEIGHT];

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

        let mut frame = [[Pixel::Transparent; SCREEN_WIDTH]; SCREEN_HEIGHT];

        for y in 0..128 {
            for x in 0..160 {
                let addr = base_addr + ((y * SCREEN_WIDTH + x) as u32 * 2);
                let rgb = self.read_u16(addr);
                frame[y][x] = Self::extract_rgb(rgb);
            }
        }

        frame
    }

    fn effective_backgrounds(&self) -> Vec<(usize, BgCnt)> {
        // we need to return a list of (index, BgCnt), or else we wont know which BGxCNT is which
        // this is important for later once we access the scroll registers as well
        // we also sort this list by priority, so that we can render the backgrounds in the correct order
        let mut bg_cnts = vec![];

        // check which backgrounds are enabled
        if self.disp_cnt.contains_flags(DispCnt::BG0_ON) {
            bg_cnts.push((0, *self.bg_cnt[0].value()));
        }

        if self.disp_cnt.contains_flags(DispCnt::BG1_ON) {
            bg_cnts.push((1, *self.bg_cnt[1].value()));
        }

        if self.disp_cnt.contains_flags(DispCnt::BG2_ON) {
            bg_cnts.push((2, *self.bg_cnt[2].value()));
        }

        if self.disp_cnt.contains_flags(DispCnt::BG3_ON) {
            bg_cnts.push((3, *self.bg_cnt[3].value()));
        }

        // sort by the provided priority, 0 is highest priority
        bg_cnts.sort_by(|a, b| a.1.priority().cmp(&b.1.priority()));
        bg_cnts.reverse(); // reverse to have the highest priority first

        bg_cnts
    }

    fn extract_rgb(rgb: u16) -> Pixel {
        let r = ((rgb & 0b0000_0000_0001_1111) as u8) << 3;
        let g = (((rgb & 0b0000_0011_1110_0000) >> 5) as u8) << 3;
        let b = (((rgb & 0b0111_1100_0000_0000) >> 10) as u8) << 3;
        Pixel::Rgb(r, g, b)
    }
}

impl Addressable for Ppu {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x04000000..=0x04000001 => self.disp_cnt.read(addr),   // DISPCNT
            0x04000004..=0x04000005 => self.disp_stat.read(addr),  // DISPSTAT
            0x04000006..=0x04000007 => self.scanline.read(addr),   // VCOUNT
            0x04000008..=0x04000009 => self.bg_cnt[0].read(addr),  // BG0CNT
            0x0400000A..=0x0400000B => self.bg_cnt[1].read(addr),  // BG1CNT
            0x0400000C..=0x0400000D => self.bg_cnt[2].read(addr),  // BG2CNT
            0x0400000E..=0x0400000F => self.bg_cnt[3].read(addr),  // BG3CNT
            0x04000010..=0x04000011 => self.bg_hofs[0].read(addr), // BG0HOFS
            0x04000012..=0x04000013 => self.bg_vofs[0].read(addr), // BG0VOFS
            0x04000014..=0x04000015 => self.bg_hofs[1].read(addr), // BG1HOFS
            0x04000016..=0x04000017 => self.bg_vofs[1].read(addr), // BG1VOFS
            0x04000018..=0x04000019 => self.bg_hofs[2].read(addr), // BG2HOFS
            0x0400001A..=0x0400001B => self.bg_vofs[2].read(addr), // BG2VOFS
            0x0400001C..=0x0400001D => self.bg_hofs[3].read(addr), // BG3HOFS
            0x0400001E..=0x0400001F => self.bg_vofs[3].read(addr), // BG3VOFS
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
            0x04000008..=0x04000009 => self.bg_cnt[0].write(addr, value), // BG0CNT
            0x0400000A..=0x0400000B => self.bg_cnt[1].write(addr, value), // BG1CNT
            0x0400000C..=0x0400000D => self.bg_cnt[2].write(addr, value), // BG2CNT
            0x0400000E..=0x0400000F => self.bg_cnt[3].write(addr, value), // BG3CNT
            0x04000010..=0x04000011 => self.bg_hofs[0].write(addr, value), // BG0HOFS
            0x04000012..=0x04000013 => self.bg_vofs[0].write(addr, value), // BG0VOFS
            0x04000014..=0x04000015 => self.bg_hofs[1].write(addr, value), // BG1HOFS
            0x04000016..=0x04000017 => self.bg_vofs[1].write(addr, value), // BG1VOFS
            0x04000018..=0x04000019 => self.bg_hofs[2].write(addr, value), // BG2HOFS
            0x0400001A..=0x0400001B => self.bg_vofs[2].write(addr, value), // BG2VOFS
            0x0400001C..=0x0400001D => self.bg_hofs[3].write(addr, value), // BG3HOFS
            0x0400001E..=0x0400001F => self.bg_vofs[3].write(addr, value), // BG3VOFS
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
