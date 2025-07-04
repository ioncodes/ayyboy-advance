pub mod ppu;
pub mod registers;
pub mod tile;

pub const SCREEN_WIDTH: usize = 240;
pub const SCREEN_HEIGHT: usize = 160;

pub const FRAME_0_ADDRESS: u32 = 0x0600_0000;
pub const FRAME_1_ADDRESS: u32 = 0x0600_A000;

pub const PALETTE_ADDR_START: u32 = 0x0500_0000;
pub const PALETTE_ADDR_END: u32 = 0x0500_03FF;

pub const PALETTE_ENTRIES: usize = 256;
pub const PALETTE_TOTAL_ENTRIES: usize = PALETTE_ENTRIES * 2; // BG and OBJ

pub const TILESET_ENTRY_SIZE: usize = 0x4000;
pub const TILEMAP_ENTRY_SIZE: usize = 0x800;

#[derive(PartialEq, Clone, Copy, Debug)]
pub enum Pixel {
    Transparent,
    Rgb(u8, u8, u8),
}

impl Pixel {
    pub fn blend(self, other: Pixel, eva: u8, evb: u8) -> Pixel {
        match (self, other) {
            (Pixel::Rgb(r1, g1, b1), Pixel::Rgb(r2, g2, b2)) => {
                let eva = eva.min(16) as u16;
                let evb = evb.min(16) as u16;
                let r = (r1 as u16 * eva + r2 as u16 * evb) / 16;
                let g = (g1 as u16 * eva + g2 as u16 * evb) / 16;
                let b = (b1 as u16 * eva + b2 as u16 * evb) / 16;
                Pixel::Rgb(r as u8, g as u8, b as u8)
            }
            _ => self,
        }
    }

    pub fn brighten(self, level: u8) -> Pixel {
        match self {
            Pixel::Rgb(r, g, b) => {
                let level = level.min(16) as u16;
                let r = r as u16 + ((255 - r as u16) * level) / 16;
                let g = g as u16 + ((255 - g as u16) * level) / 16;
                let b = b as u16 + ((255 - b as u16) * level) / 16;
                Pixel::Rgb(r as u8, g as u8, b as u8)
            }
            x => x,
        }
    }

    pub fn darken(self, level: u8) -> Pixel {
        match self {
            Pixel::Rgb(r, g, b) => {
                let level = level.min(16) as u16;
                let r = r as u16 - (r as u16 * level) / 16;
                let g = g as u16 - (g as u16 * level) / 16;
                let b = b as u16 - (b as u16 * level) / 16;
                Pixel::Rgb(r as u8, g as u8, b as u8)
            }
            x => x,
        }
    }
}

pub type Frame = [[Pixel; SCREEN_WIDTH]; SCREEN_HEIGHT];
