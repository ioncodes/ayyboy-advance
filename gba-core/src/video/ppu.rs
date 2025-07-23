use super::registers::{BgCnt, BgOffset, ColorDepth, DispCnt, DispStat, ObjShape};
use super::tile::Tile;
use super::{Frame, PALETTE_ADDR_END, PALETTE_ADDR_START, PALETTE_TOTAL_ENTRIES, Pixel, SCREEN_HEIGHT, SCREEN_WIDTH};
use crate::memory::device::{Addressable, IoRegister};
use crate::video::TILEMAP_ENTRY_SIZE;
use crate::video::registers::{
    BgAffineParam, BgRefPointHigh, BgRefPointLow, BldAlpha, BldCnt, BldY, Dimension, InternalScreenSize, ObjAttribute0,
    ObjAttribute1, ObjAttribute2, ObjSize, Sfx, WindowControl, WindowDimensions,
};
use crate::video::tile::TileInfo;
use tracing::*;

#[derive(Clone, Copy, PartialEq)]
enum WindowRegion {
    Win0,
    Win1,
    Outside,
}

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
    pub attr0: ObjAttribute0,
    pub attr1: ObjAttribute1,
    pub attr2: ObjAttribute2,
    pub attr0_addr: u32,
    pub attr1_addr: u32,
    pub attr2_addr: u32,
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
    pub bg_pa: [IoRegister<BgAffineParam>; 2],
    pub bg_pb: [IoRegister<BgAffineParam>; 2],
    pub bg_pc: [IoRegister<BgAffineParam>; 2],
    pub bg_pd: [IoRegister<BgAffineParam>; 2],
    pub bg_refx_l: [IoRegister<BgRefPointLow>; 2],
    pub bg_refx_h: [IoRegister<BgRefPointHigh>; 2],
    pub bg_refy_l: [IoRegister<BgRefPointLow>; 2],
    pub bg_refy_h: [IoRegister<BgRefPointHigh>; 2],
    pub win0_h: IoRegister<WindowDimensions>,
    pub win1_h: IoRegister<WindowDimensions>,
    pub win0_v: IoRegister<WindowDimensions>,
    pub win1_v: IoRegister<WindowDimensions>,
    pub winin: IoRegister<WindowControl>,
    pub winout: IoRegister<WindowControl>,
    pub bld_cnt: IoRegister<BldCnt>,
    pub bld_alpha: IoRegister<BldAlpha>,
    pub bld_y: IoRegister<BldY>,
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
            bg_pa: [IoRegister::default(); 2],
            bg_pb: [IoRegister::default(); 2],
            bg_pc: [IoRegister::default(); 2],
            bg_pd: [IoRegister::default(); 2],
            bg_refx_l: [IoRegister::default(); 2],
            bg_refx_h: [IoRegister::default(); 2],
            bg_refy_l: [IoRegister::default(); 2],
            bg_refy_h: [IoRegister::default(); 2],
            win0_h: IoRegister::default(),
            win1_h: IoRegister::default(),
            win0_v: IoRegister::default(),
            win1_v: IoRegister::default(),
            winin: IoRegister::default(),
            winout: IoRegister::default(),
            bld_cnt: IoRegister::default(),
            bld_alpha: IoRegister::default(),
            bld_y: IoRegister::default(),
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
        trace!(target: "ppu", "Grabbing internal frame buffer for PPU mode: {}", lcd_control.bg_mode());

        let sprite_layer = self.render_sprites();

        let bg_layers = match lcd_control.bg_mode() {
            0 => self.render_background_mode0_layers(),
            1..=2 => self.render_background_mode0_layers(), // TODO: should prob not deal with these modes inside of mode0
            3..=5 => {
                let mut layers = vec![[[Pixel::Transparent; SCREEN_WIDTH]; SCREEN_HEIGHT]; 4];
                match lcd_control.bg_mode() {
                    3 => {
                        layers[2] = self.render_background_mode3(lcd_control.frame_address());
                    }
                    4 => {
                        layers[2] = self.render_background_mode4(lcd_control.frame_address());
                    }
                    5 => {
                        layers[2] = self.render_background_mode5(lcd_control.frame_address());
                    }
                    _ => unreachable!(),
                }
                layers
            }
            _ => unreachable!(),
        };

        self.compose_layers(&bg_layers, &sprite_layer)
    }

    pub fn get_background_frame(&self, mode: usize, base_addr: u32) -> Frame {
        match mode {
            0 => {
                let layers = self.render_background_mode0_layers();
                self.compose_layers(&layers, &vec![(5, Pixel::Transparent); SCREEN_WIDTH * SCREEN_HEIGHT])
            }
            1..=2 => {
                let layers = self.render_background_mode0_layers();
                self.compose_layers(&layers, &vec![(5, Pixel::Transparent); SCREEN_WIDTH * SCREEN_HEIGHT])
            }
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
        let w_px = TILES_PER_ROW * TILE_WIDTH; // atlas width in px (128)
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

        assert_eq!(
            out.len(),
            w_px * h_px,
            "Tileset size mismatch: {} != {}",
            out.len(),
            w_px * h_px
        );

        (tile_count, out)
    }

    pub fn render_tilemap(&self, bg: usize, bg_cnt: &BgCnt) -> (InternalScreenSize, Vec<Pixel>) {
        let palette = self.fetch_palette();

        let tileset_addr = bg_cnt.tileset_addr() as usize; // cbb
        let tilemap_addr = bg_cnt.tilemap_addr() as usize; // sbb

        let tile_size = match bg_cnt.bpp() {
            ColorDepth::Bpp4 => 0x20,
            ColorDepth::Bpp8 => 0x40,
        };

        let bg_mode = self.disp_cnt.value().bg_mode();
        let (map_w, map_h, tiles_x, tiles_y) = match bg_cnt.screen_size(bg, bg_mode) {
            InternalScreenSize::Text256x256 => (256, 256, 32, 32),
            InternalScreenSize::Text512x256 => (512, 256, 64, 32),
            InternalScreenSize::Text256x512 => (256, 512, 32, 64),
            InternalScreenSize::Text512x512 => (512, 512, 64, 64),

            InternalScreenSize::Affine128x128 => (128, 128, 16, 16),
            InternalScreenSize::Affine256x256 => (256, 256, 32, 32),
            InternalScreenSize::Affine512x512 => (512, 512, 64, 64),
            InternalScreenSize::Affine1024x1024 => (1024, 1024, 128, 128),
        };

        let screen_size = bg_cnt.screen_size(bg, bg_mode);
        let is_text_mode = matches!(
            screen_size,
            InternalScreenSize::Text256x256
                | InternalScreenSize::Text512x256
                | InternalScreenSize::Text256x512
                | InternalScreenSize::Text512x512
        );

        let mut internal_frame = vec![Pixel::Transparent; map_w * map_h];

        for ty in 0..tiles_y {
            for tx in 0..tiles_x {
                let addr = if is_text_mode {
                    let (block_col, block_row) = (tx / 32, ty / 32); // which 32×32 map
                    let (local_col, local_row) = (tx & 31, ty & 31); // pos inside that map

                    let block_index = match screen_size {
                        InternalScreenSize::Text256x256 => 0,                         // SC0
                        InternalScreenSize::Text512x256 => block_col,                 // SC0‥SC1
                        InternalScreenSize::Text256x512 => block_row,                 // SC0‥SC1
                        InternalScreenSize::Text512x512 => block_row * 2 + block_col, // SC0‥SC3

                        InternalScreenSize::Affine128x128
                        | InternalScreenSize::Affine256x256
                        | InternalScreenSize::Affine512x512
                        | InternalScreenSize::Affine1024x1024 => 0,
                    };

                    // fetch the tile from the tilemap
                    (tilemap_addr + (block_index * TILEMAP_ENTRY_SIZE) + (local_row * 32 + local_col) * 2) as u32
                } else {
                    (tilemap_addr + (ty * tiles_x + tx)) as u32
                };

                let entry = if is_text_mode {
                    self.read_u16(addr as u32)
                } else {
                    self.read(addr as u32) as u16
                };
                let tile_info = TileInfo::from_bits_truncate(entry);

                // fetch the tile data from the tileset
                let tile_addr = tileset_addr + tile_info.tile_id(is_text_mode) * tile_size;
                let tile_data = {
                    let mut tile_data = vec![0u8; tile_size];
                    for i in 0..tile_size {
                        tile_data[i] = self.read((tile_addr + i) as u32);
                    }
                    tile_data
                };

                // extract the tile pixels using the given palette bank
                let palette_bank = if tile_size == 0x20 {
                    &palette[tile_info.palette() * 16..][..16]
                } else {
                    &palette[..256]
                };
                let mut tile = Tile::from_bytes(&tile_data, palette_bank);

                // flip the tile if needed
                if is_text_mode {
                    if tile_info.contains(TileInfo::FLIP_X) {
                        tile.flip_x();
                    }

                    if tile_info.contains(TileInfo::FLIP_Y) {
                        tile.flip_y();
                    }
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

        assert_eq!(
            internal_frame.len(),
            map_w * map_h,
            "Internal frame size mismatch: {} != {}",
            internal_frame.len(),
            map_w * map_h
        );

        (screen_size, internal_frame)
    }

    pub fn create_sprite_debug_map(&self) -> Vec<Sprite> {
        const OAM_BASE: u32 = 0x0700_0000;
        const OBJ_BASE: u32 = 0x0601_0000;
        const CHAR_UNIT_SIZE: u32 = 32;

        let mut sprites = Vec::with_capacity(128);

        let palette = self.fetch_palette();
        let obj_palette = &palette[256..512];
        let obj_dimension = self.disp_cnt.value().dimension();
        let bg_mode = self.disp_cnt.value().bg_mode();

        for obj_id in 0..128 {
            let attr0_addr = OAM_BASE + (obj_id * 8) + 0;
            let attr1_addr = OAM_BASE + (obj_id * 8) + 2;
            let attr2_addr = OAM_BASE + (obj_id * 8) + 4;

            let attr0 = ObjAttribute0::from_bits_truncate(self.read_u16(attr0_addr));
            let attr1 = ObjAttribute1::from_bits_truncate(self.read_u16(attr1_addr));
            let attr2 = ObjAttribute2::from_bits_truncate(self.read_u16(attr2_addr));

            let shape = attr0.shape();
            let size = attr1.size(shape);
            let (w_px, h_px) = Self::obj_dimensions(shape, size);
            if w_px == 0 {
                continue;
            }

            let tiles_x = w_px / 8;
            let tiles_y = h_px / 8;
            let bpp_factor = if attr0.bpp() == ColorDepth::Bpp8 { 2 } else { 1 };
            let row_stride = if obj_dimension == Dimension::OneDimensional {
                tiles_x * bpp_factor
            } else {
                32
            };
            let char_num_base = if attr0.bpp() == ColorDepth::Bpp8 {
                (attr2.tile_number() & !1) as u32
            } else {
                attr2.tile_number() as u32
            };

            let tile_size = if attr0.bpp() == ColorDepth::Bpp8 { 64 } else { 32 };
            let mut sprite_data = vec![Pixel::Transparent; w_px * h_px];

            for ty in 0..tiles_y {
                for tx in 0..tiles_x {
                    let src_tx = if attr1.x_flip() { tiles_x - 1 - tx } else { tx };
                    let src_ty = if attr1.y_flip() { tiles_y - 1 - ty } else { ty };

                    let char_offset = (src_ty * row_stride + src_tx * bpp_factor) as u32;
                    let tile_nr = char_num_base + char_offset;

                    if (3..=5).contains(&bg_mode) && tile_nr < 512 {
                        continue;
                    }

                    let tile_addr = OBJ_BASE + tile_nr * CHAR_UNIT_SIZE;

                    // fetch raw tile bytes
                    let mut tile_bytes = [0u8; 64];
                    for i in 0..tile_size {
                        tile_bytes[i] = self.read(tile_addr + i as u32);
                    }

                    // palette slice
                    let pal_slice = if attr0.bpp() == ColorDepth::Bpp4 {
                        &obj_palette[attr2.palette() * 16..][..16]
                    } else {
                        &palette[256..512]
                    };
                    let mut tile = Tile::from_bytes(&tile_bytes[..tile_size], pal_slice);

                    if attr1.x_flip() {
                        tile.flip_x();
                    }
                    if attr1.y_flip() {
                        tile.flip_y();
                    }

                    // blit into per-sprite buffer
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
                attr0,
                attr1,
                attr2,
                attr0_addr,
                attr1_addr,
                attr2_addr,
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
                    ObjSize::Vertical8x16 | ObjSize::Vertical8x32 | ObjSize::Vertical16x32 | ObjSize::Vertical32x64
                ),
            },
            "ObjShape({:?}) and ObjSize({:?}) mismatch",
            shape,
            size
        );

        dims
    }

    fn render_sprites(&self) -> Vec<(usize, Pixel)> {
        const OAM_BASE: u32 = 0x0700_0000;
        const OBJ_BASE: u32 = 0x0601_0000;
        const CHAR_UNIT_SIZE: u32 = 32;

        let mut frame = vec![(5, Pixel::Transparent); SCREEN_WIDTH * SCREEN_HEIGHT];

        let lcd_control = self.disp_cnt.value();
        let bg_mode = lcd_control.bg_mode();

        let palette = self.fetch_palette();
        let obj_palette = &palette[256..512];

        let obj_dimension = self.disp_cnt.value().dimension();

        // lower OAM entry = higher priority
        // quick hack is to go through the OAM backwards
        for obj_id in (0..128).rev() {
            let attr0_addr = OAM_BASE + (obj_id * 8) + 0;
            let attr1_addr = OAM_BASE + (obj_id * 8) + 2;
            let attr2_addr = OAM_BASE + (obj_id * 8) + 4;

            let attr0 = ObjAttribute0::from_bits_truncate(self.read_u16(attr0_addr));
            let attr1 = ObjAttribute1::from_bits_truncate(self.read_u16(attr1_addr));
            let attr2 = ObjAttribute2::from_bits_truncate(self.read_u16(attr2_addr));

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

            // tiles per dimension
            let tiles_x = w_px / 8;
            let tiles_y = h_px / 8;

            let bpp_factor = if attr0.bpp() == ColorDepth::Bpp8 { 2 } else { 1 };
            let row_stride = if obj_dimension == Dimension::OneDimensional {
                tiles_x * bpp_factor
            } else {
                32
            };

            let tile_size = if attr0.bpp() == ColorDepth::Bpp8 { 0x40 } else { 0x20 };

            for ty in 0..tiles_y {
                for tx in 0..tiles_x {
                    let src_tx = if attr1.x_flip() { tiles_x - 1 - tx } else { tx };
                    let src_ty = if attr1.y_flip() { tiles_y - 1 - ty } else { ty };

                    let char_num_base = if attr0.bpp() == ColorDepth::Bpp8 {
                        (attr2.tile_number() & !1) as u32 // even-align for 256-colour mode
                    } else {
                        attr2.tile_number() as u32 // leave 4-bpp numbers untouched
                    };
                    let char_offset = (src_ty * row_stride + src_tx * bpp_factor) as u32;
                    let tile_nr = char_num_base + char_offset;

                    // https://problemkaputt.de/gbatek.htm#lcdobjoamattributes
                    // 2. When using BG Mode 3-5 (Bitmap Modes), only tile numbers 512-1023 may be used.
                    // That is because lower 16K of OBJ memory are used for BG. Attempts to use tiles 0-511 are ignored (not displayed).
                    if (3..=5).contains(&bg_mode) && tile_nr < 512 {
                        continue;
                    }

                    let tile_addr = OBJ_BASE + (tile_nr * CHAR_UNIT_SIZE);

                    // fetch raw tile bytes
                    let mut tile_data = [0u8; 64]; // overcommit to avoid vec! allocation
                    for i in 0..tile_size {
                        tile_data[i] = self.read(tile_addr + i as u32);
                    }

                    // extract the tile pixels using the given palette bank
                    let pal_slice = if attr0.bpp() == ColorDepth::Bpp4 {
                        &obj_palette[attr2.palette() * 16..][..16]
                    } else {
                        &palette[256..512]
                    };
                    let mut tile = Tile::from_bytes(&tile_data[..tile_size], pal_slice);

                    // flip the tile if needed
                    if !attr0.is_affine() {
                        if attr1.x_flip() {
                            tile.flip_x();
                        }
                        if attr1.y_flip() {
                            tile.flip_y();
                        }
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

                            let color = tile.pixels[py * 8 + px];
                            if color != Pixel::Transparent {
                                let sprite_idx = (sy as usize) * SCREEN_WIDTH + (sx as usize);
                                frame[sprite_idx] = (attr2.priority(), color);
                            }
                        }
                    }
                }
            }
        }

        frame
    }

    fn render_background_mode0_layers(&self) -> Vec<Frame> {
        trace!(target: "ppu", "Rendering background mode 0 layers");

        let mut layers = vec![[[Pixel::Transparent; SCREEN_WIDTH]; SCREEN_HEIGHT]; 4];
        let bg_mode = self.disp_cnt.value().bg_mode();

        for id in 0..4 {
            let enabled = match id {
                0 => self.disp_cnt.contains_flags(DispCnt::BG0_ON),
                1 => self.disp_cnt.contains_flags(DispCnt::BG1_ON),
                2 => self.disp_cnt.contains_flags(DispCnt::BG2_ON),
                3 => self.disp_cnt.contains_flags(DispCnt::BG3_ON),
                _ => false,
            };

            if !enabled {
                continue;
            }

            let bg_cnt = self.bg_cnt[id].value();
            let screen_size = bg_cnt.screen_size(id, bg_mode);
            let (map_w, map_h) = (screen_size.width(), screen_size.height());

            let is_affine = matches!(
                screen_size,
                InternalScreenSize::Affine128x128
                    | InternalScreenSize::Affine256x256
                    | InternalScreenSize::Affine512x512
                    | InternalScreenSize::Affine1024x1024
            );

            let (_, tilemap) = self.render_tilemap(id, &bg_cnt);

            if is_affine {
                let i = id - 2; // BG2=0, BG3=1
                let pa = self.bg_pa[i].value().bits() as i32;
                let pb = self.bg_pb[i].value().bits() as i32;
                let pc = self.bg_pc[i].value().bits() as i32;
                let pd = self.bg_pd[i].value().bits() as i32;
                let refx = self.bg_refx_h[i].value().full_value(self.bg_refx_l[i].value());
                let refy = self.bg_refy_h[i].value().full_value(self.bg_refy_l[i].value());
                let wrap = !bg_cnt.contains(BgCnt::DISPLAY_OVERFLOW);

                for y in 0..SCREEN_HEIGHT {
                    for x in 0..SCREEN_WIDTH {
                        let fx = refx + pa * x as i32 + pb * y as i32;
                        let fy = refy + pc * x as i32 + pd * y as i32;
                        let mut sx = (fx >> 8) as i32;
                        let mut sy = (fy >> 8) as i32;

                        if wrap {
                            sx = sx.rem_euclid(map_w as i32);
                            sy = sy.rem_euclid(map_h as i32);
                        } else if sx < 0 || sx >= map_w as i32 || sy < 0 || sy >= map_h as i32 {
                            continue;
                        }

                        let color = tilemap[(sy as usize) * map_w + (sx as usize)];
                        if color != Pixel::Transparent {
                            layers[id][y][x] = color;
                        }
                    }
                }
            } else {
                let vertical_offset = self.bg_vofs[id].value().offset();
                let horizontal_offset = self.bg_hofs[id].value().offset();

                let hoff = horizontal_offset % map_w;
                let voff = vertical_offset % map_h;

                for y in 0..SCREEN_HEIGHT {
                    let src_y = (y + voff) % map_h;
                    for x in 0..SCREEN_WIDTH {
                        let src_x = (x + hoff) % map_w;
                        let color = tilemap[src_y * map_w + src_x];
                        if color != Pixel::Transparent {
                            layers[id][y][x] = color;
                        }
                    }
                }
            }
        }

        layers
    }

    fn render_background_mode3(&self, base_addr: u32) -> Frame {
        trace!(target: "ppu", "Rendering background mode 3 @ {:08X}", base_addr);

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
        trace!(target: "ppu", "Rendering background mode 4 @ {:08X}", base_addr);

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
        trace!(target: "ppu", "Rendering background mode 5 @ {:08X}", base_addr);

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

    fn point_in_window(&self, x: usize, y: usize, h: &WindowDimensions, v: &WindowDimensions) -> bool {
        let (x1, x2) = (h.x1(), h.x2());
        let (y1, y2) = (v.x1(), v.x2());

        let inside_x = if x1 <= x2 { x >= x1 && x < x2 } else { x >= x1 || x < x2 };
        let inside_y = if y1 <= y2 { y >= y1 && y < y2 } else { y >= y1 || y < y2 };

        inside_x && inside_y
    }

    fn window_region_for_pixel(&self, x: usize, y: usize) -> WindowRegion {
        let disp = self.disp_cnt.value();

        if disp.contains(DispCnt::WIN0_ON) && self.point_in_window(x, y, self.win0_h.value(), self.win0_v.value()) {
            return WindowRegion::Win0;
        }

        if disp.contains(DispCnt::WIN1_ON) && self.point_in_window(x, y, self.win1_h.value(), self.win1_v.value()) {
            return WindowRegion::Win1;
        }

        WindowRegion::Outside
    }

    fn compose_layers(&self, bg_layers: &Vec<Frame>, sprite_frame: &Vec<(usize, Pixel)>) -> Frame {
        assert_eq!(bg_layers.len(), 4, "Expected 4 background layers");

        let palette = self.fetch_palette();
        let backdrop = palette[0];
        let mut frame = [[backdrop; SCREEN_WIDTH]; SCREEN_HEIGHT];

        let winin = self.winin.value();
        let winout = self.winout.value();

        let win0_on = self.disp_cnt.value().contains(DispCnt::WIN0_ON);
        let win1_on = self.disp_cnt.value().contains(DispCnt::WIN1_ON);
        let objwin_on = self.disp_cnt.value().contains(DispCnt::OBJ_WIN_ON);
        let windows_active = win0_on || win1_on || objwin_on;

        let master_bg = [
            self.disp_cnt.value().contains(DispCnt::BG0_ON),
            self.disp_cnt.value().contains(DispCnt::BG1_ON),
            self.disp_cnt.value().contains(DispCnt::BG2_ON),
            self.disp_cnt.value().contains(DispCnt::BG3_ON),
        ];
        let master_obj = self.disp_cnt.value().contains(DispCnt::OBJ_ON);

        let bg_enabled = |region: WindowRegion, id: usize| -> bool {
            if !master_bg[id] {
                return false;
            }

            if !windows_active {
                return true;
            }

            match region {
                WindowRegion::Win0 => winin.is_bg_enabled_win0(id),
                WindowRegion::Win1 => winin.is_bg_enabled_win1(id),
                WindowRegion::Outside => winout.is_bg_enabled_out(id),
            }
        };

        let obj_enabled = |region: WindowRegion| -> bool {
            if !master_obj {
                return false;
            }

            if !windows_active {
                return true;
            }

            match region {
                WindowRegion::Win0 => winin.obj_enabled_win0(),
                WindowRegion::Win1 => winin.obj_enabled_win1(),
                WindowRegion::Outside => winout.obj_enabled_out(),
            }
        };
        let bg_mode = self.disp_cnt.value().bg_mode();

        let bg_priorities = [
            self.bg_cnt[0].value().priority(),
            self.bg_cnt[1].value().priority(),
            self.bg_cnt[2].value().priority(),
            self.bg_cnt[3].value().priority(),
        ];

        // Determine which backgrounds to process based on mode
        let (start_bg, end_bg) = if bg_mode >= 3 { (2, 2) } else { (0, 3) };

        for y in 0..SCREEN_HEIGHT {
            let sprite_row_start = y * SCREEN_WIDTH;
            let frame_row = &mut frame[y];

            for x in 0..SCREEN_WIDTH {
                let region = self.window_region_for_pixel(x, y);

                // Collect visible surfaces at this pixel
                let mut surfaces: Vec<(usize, Pixel, usize, usize)> = Vec::new();

                // Backdrop always present
                surfaces.push((5, backdrop, 4, 5));

                // Background layers
                for id in start_bg..=end_bg {
                    if !bg_enabled(region, id) {
                        continue;
                    }

                    let layer_color = bg_layers[id][y][x];
                    if layer_color != Pixel::Transparent {
                        let priority = bg_priorities[id];
                        let order = id + 1; // BG0=1 .. BG3=4
                        surfaces.push((id, layer_color, priority, order));
                    }
                }

                // Sprite layer
                let sprite_idx = sprite_row_start + x;
                let (sprite_priority, sprite_color) = sprite_frame[sprite_idx];
                if obj_enabled(region) && sprite_color != Pixel::Transparent {
                    surfaces.push((4, sprite_color, sprite_priority, 0));
                }

                // Sort by priority then order
                surfaces.sort_by(|a, b| match a.2.cmp(&b.2) {
                    std::cmp::Ordering::Equal => a.3.cmp(&b.3),
                    ord => ord,
                });

                let (top_layer, top_color, _, _) = surfaces[0];
                let second = surfaces.get(1).copied().unwrap_or((5, Pixel::Transparent, 4, 5));
                let (second_layer, second_color, _, _) = second;

                let bld_cnt = self.bld_cnt.value();
                let final_color = match bld_cnt.sfx() {
                    Sfx::AlphaBlend => {
                        if bld_cnt.is_first_target(top_layer) && bld_cnt.is_second_target(second_layer) {
                            top_color.blend(second_color, self.bld_alpha.value().eva(), self.bld_alpha.value().evb())
                        } else {
                            top_color
                        }
                    }
                    Sfx::IncreaseBrightness => {
                        if bld_cnt.is_first_target(top_layer) {
                            top_color.brighten(self.bld_y.value().evy())
                        } else {
                            top_color
                        }
                    }
                    Sfx::DecreaseBrightness => {
                        if bld_cnt.is_first_target(top_layer) {
                            top_color.darken(self.bld_y.value().evy())
                        } else {
                            top_color
                        }
                    }
                    Sfx::None => top_color,
                };
                frame_row[x] = final_color;
            }
        }

        frame
    }

    fn extract_rgb(rgb: u16) -> Pixel {
        let r5 = (rgb & 0x001F) as u8;
        let g5 = ((rgb >> 5) & 0x001F) as u8;
        let b5 = ((rgb >> 10) & 0x001F) as u8;

        let r = (r5 << 3) | (r5 >> 2);
        let g = (g5 << 3) | (g5 >> 2);
        let b = (b5 << 3) | (b5 >> 2);

        Pixel::Rgb(r, g, b)
    }
}

impl Addressable for Ppu {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x04000000..=0x04000001 => self.disp_cnt.read(addr),     // DISPCNT
            0x04000004..=0x04000005 => self.disp_stat.read(addr),    // DISPSTAT
            0x04000006..=0x04000007 => self.scanline.read(addr),     // VCOUNT
            0x04000008..=0x04000009 => self.bg_cnt[0].read(addr),    // BG0CNT
            0x0400000A..=0x0400000B => self.bg_cnt[1].read(addr),    // BG1CNT
            0x0400000C..=0x0400000D => self.bg_cnt[2].read(addr),    // BG2CNT
            0x0400000E..=0x0400000F => self.bg_cnt[3].read(addr),    // BG3CNT
            0x04000010..=0x04000011 => self.bg_hofs[0].read(addr),   // BG0HOFS
            0x04000012..=0x04000013 => self.bg_vofs[0].read(addr),   // BG0VOFS
            0x04000014..=0x04000015 => self.bg_hofs[1].read(addr),   // BG1HOFS
            0x04000016..=0x04000017 => self.bg_vofs[1].read(addr),   // BG1VOFS
            0x04000018..=0x04000019 => self.bg_hofs[2].read(addr),   // BG2HOFS
            0x0400001A..=0x0400001B => self.bg_vofs[2].read(addr),   // BG2VOFS
            0x0400001C..=0x0400001D => self.bg_hofs[3].read(addr),   // BG3HOFS
            0x0400001E..=0x0400001F => self.bg_vofs[3].read(addr),   // BG3VOFS
            0x04000020..=0x04000021 => self.bg_pa[0].read(addr),     // BG2PA
            0x04000022..=0x04000023 => self.bg_pb[0].read(addr),     // BG2PB
            0x04000024..=0x04000025 => self.bg_pc[0].read(addr),     // BG2PC
            0x04000026..=0x04000027 => self.bg_pd[0].read(addr),     // BG2PD
            0x04000028..=0x04000029 => self.bg_refx_l[0].read(addr), // BG2X_L
            0x0400002A..=0x0400002B => self.bg_refx_h[0].read(addr), // BG2X_H
            0x0400002C..=0x0400002D => self.bg_refy_l[0].read(addr), // BG2Y_L
            0x0400002E..=0x0400002F => self.bg_refy_h[0].read(addr), // BG2Y_H
            0x04000030..=0x04000031 => self.bg_pa[1].read(addr),     // BG3PA
            0x04000032..=0x04000033 => self.bg_pb[1].read(addr),     // BG3PB
            0x04000034..=0x04000035 => self.bg_pc[1].read(addr),     // BG3PC
            0x04000036..=0x04000037 => self.bg_pd[1].read(addr),     // BG3PD
            0x04000038..=0x04000039 => self.bg_refx_l[1].read(addr), // BG3X_L
            0x0400003A..=0x0400003B => self.bg_refx_h[1].read(addr), // BG3X_H
            0x0400003C..=0x0400003D => self.bg_refy_l[1].read(addr), // BG3Y_L
            0x0400003E..=0x0400003F => self.bg_refy_h[1].read(addr), // BG3Y_H
            0x04000040..=0x04000041 => self.win0_h.read(addr),       // WIN0H
            0x04000042..=0x04000043 => self.win1_h.read(addr),       // WIN1H
            0x04000044..=0x04000045 => self.win0_v.read(addr),       // WIN0V
            0x04000046..=0x04000047 => self.win1_v.read(addr),       // WIN1V
            0x04000048..=0x04000049 => self.winin.read(addr),        // WININ
            0x0400004A..=0x0400004B => self.winout.read(addr),       // WINOUT
            0x04000050..=0x04000051 => self.bld_cnt.read(addr),      // BLDCNT
            0x04000052..=0x04000053 => self.bld_alpha.read(addr),    // BLDALPHA
            0x04000054..=0x04000054 => self.bld_y.read(addr),        // BLDY
            // rest of the registers
            0x04000000..=0x04000056 => {
                error!(target: "ppu", "Reading from unmapped I/O address: {:08X}", addr);
                self.io[(addr - 0x04000000) as usize]
            }
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
            0x04000020..=0x04000021 => self.bg_pa[0].write(addr, value), // BG2PA
            0x04000022..=0x04000023 => self.bg_pb[0].write(addr, value), // BG2PB
            0x04000024..=0x04000025 => self.bg_pc[0].write(addr, value), // BG2PC
            0x04000026..=0x04000027 => self.bg_pd[0].write(addr, value), // BG2PD
            0x04000028..=0x04000029 => self.bg_refx_l[0].write(addr, value), // BG2X_L
            0x0400002A..=0x0400002B => self.bg_refx_h[0].write(addr, value), // BG2X_H
            0x0400002C..=0x0400002D => self.bg_refy_l[0].write(addr, value), // BG2Y_L
            0x0400002E..=0x0400002F => self.bg_refy_h[0].write(addr, value), // BG2Y_H
            0x04000030..=0x04000031 => self.bg_pa[1].write(addr, value), // BG3PA
            0x04000032..=0x04000033 => self.bg_pb[1].write(addr, value), // BG3PB
            0x04000034..=0x04000035 => self.bg_pc[1].write(addr, value), // BG3PC
            0x04000036..=0x04000037 => self.bg_pd[1].write(addr, value), // BG3PD
            0x04000038..=0x04000039 => self.bg_refx_l[1].write(addr, value), // BG3X_L
            0x0400003A..=0x0400003B => self.bg_refx_h[1].write(addr, value), // BG3X_H
            0x0400003C..=0x0400003D => self.bg_refy_l[1].write(addr, value), // BG3Y_L
            0x0400003E..=0x0400003F => self.bg_refy_h[1].write(addr, value), // BG3Y_H
            0x04000040..=0x04000041 => self.win0_h.write(addr, value),   // WIN0H
            0x04000042..=0x04000043 => self.win1_h.write(addr, value),   // WIN1H
            0x04000044..=0x04000045 => self.win0_v.write(addr, value),   // WIN0V
            0x04000046..=0x04000047 => self.win1_v.write(addr, value),   // WIN1V
            0x04000048..=0x04000049 => self.winin.write(addr, value),    // WININ
            0x0400004A..=0x0400004B => self.winout.write(addr, value),   // WINOUT
            0x04000050..=0x04000051 => self.bld_cnt.write(addr, value),  // BLDCNT
            0x04000052..=0x04000053 => self.bld_alpha.write(addr, value), // BLDALPHA
            0x04000054..=0x04000054 => self.bld_y.write(addr, value),    // BLDY
            // rest of the registers
            0x04000000..=0x04000056 => {
                error!(target: "ppu", "Writing to unmapped I/O address: {:08X} with value: {:02X}", addr, value);
                self.io[(addr - 0x04000000) as usize] = value
            }
            0x05000000..=0x07FFFFFF => {
                trace!(target: "ppu", "Writing to VRAM address: {:08X} with value: {:02X}", addr, value);
                self.vram[(addr - 0x05000000) as usize] = value
            }
            _ => unreachable!(),
        }
    }
}
