use super::Rgb;
use bitflags::bitflags;

#[derive(Clone, Copy)]
pub struct Tile {
    pub pixels: [Rgb; 64],
}

impl Tile {
    pub fn from_bytes(bytes: &[u8], palette: &[Rgb]) -> Self {
        assert!(
            bytes.len() == 0x20 || bytes.len() == 0x40,
            "Tile data must be 32 or 64 bytes long, got {} bytes",
            bytes.len()
        );

        let pixels = if bytes.len() == 0x20 {
            Self::parse_as_4bpp(bytes, palette)
        } else {
            Self::parse_as_8bpp(bytes, palette)
        };

        Tile { pixels }
    }

    pub fn flip_x(&mut self) {
        for y in 0..8 {
            for x in 0..4 {
                let left_index = y * 8 + x;
                let right_index = y * 8 + (7 - x);
                self.pixels.swap(left_index, right_index);
            }
        }
    }

    pub fn flip_y(&mut self) {
        for y in 0..4 {
            for x in 0..8 {
                let top_index = y * 8 + x;
                let bottom_index = (7 - y) * 8 + x;
                self.pixels.swap(top_index, bottom_index);
            }
        }
    }

    fn parse_as_4bpp(bytes: &[u8], palette: &[Rgb]) -> [Rgb; 64] {
        let mut pixels = [(0, 0, 0); 64];

        for i in 0..32 {
            let left_pixel = (bytes[i] & 0xF0) >> 4;
            let right_pixel = bytes[i] & 0x0F;

            let idx = i * 2;
            if left_pixel != 0 {
                pixels[idx] = palette[left_pixel as usize];
            } else {
                pixels[idx] = (0, 0, 0); // Transparent pixel
            }

            if right_pixel != 0 {
                pixels[idx + 1] = palette[right_pixel as usize];
            } else {
                pixels[idx + 1] = (0, 0, 0); // Transparent pixel
            }
        }

        pixels
    }

    fn parse_as_8bpp(bytes: &[u8], palette: &[Rgb]) -> [Rgb; 64] {
        let mut pixels = [(0, 0, 0); 64];

        for i in 0..64 {
            let color_index = bytes[i];
            pixels[i] = palette[color_index as usize];
        }

        pixels
    }
}

impl Default for Tile {
    fn default() -> Self {
        Tile {
            pixels: [(0, 0, 0); 64],
        }
    }
}

pub struct TileSet {
    pub tiles: Vec<Tile>,
}

impl TileSet {
    pub fn new(size: usize) -> Self {
        TileSet {
            tiles: vec![Tile::default(); size],
        }
    }

    pub fn add_tile(&mut self, x: usize, y: usize, tile: Tile) {
        self.tiles[y * 64 + x] = tile;
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Rgb {
        let tile_x = x / 8;
        let tile_y = y / 8;
        let pixel_x = x % 8;
        let pixel_y = y % 8;

        let tile_index = tile_y * 64 + tile_x;
        self.tiles[tile_index].pixels[pixel_y * 8 + pixel_x]
    }
}

bitflags! {
    pub struct TileInfo: u16 {
        const TILE_ID = 0b0000_0011_1111_1111;
        const FLIP_X  = 0b0000_0100_0000_0000;
        const FLIP_Y  = 0b0000_1000_0000_0000;
        const PALETTE = 0b1111_0000_0000_0000;
    }
}

impl TileInfo {
    pub fn tile_id(&self) -> usize {
        (self.bits() & TileInfo::TILE_ID.bits()) as usize
    }

    pub fn flip_x(&self) -> bool {
        self.contains(TileInfo::FLIP_X)
    }

    pub fn flip_y(&self) -> bool {
        self.contains(TileInfo::FLIP_Y)
    }

    pub fn palette(&self) -> usize {
        ((self.bits() & TileInfo::PALETTE.bits()) >> 12) as usize
    }
}
