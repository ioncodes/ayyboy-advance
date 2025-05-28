use super::Rgb;

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

    fn parse_as_4bpp(bytes: &[u8], palette: &[Rgb]) -> [Rgb; 64] {
        let mut pixels = [(0, 0, 0); 64];

        for i in 0..32 {
            let left_pixel = (bytes[i] & 0xF0) >> 4;
            let right_pixel = bytes[i] & 0x0F;
            let left_color = palette[left_pixel as usize];
            let right_color = palette[right_pixel as usize];

            let idx = i * 2;
            pixels[idx + 0] = (left_color.0, left_color.1, left_color.2);
            pixels[idx + 1] = (right_color.0, right_color.1, right_color.2);
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
