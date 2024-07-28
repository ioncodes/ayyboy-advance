pub mod ppu;
mod registers;

pub const SCREEN_WIDTH: usize = 240;
pub const SCREEN_HEIGHT: usize = 160;
pub const INTERNAL_WIDTH: usize = 256;
pub const INTERNAL_HEIGHT: usize = 256;

pub type Pixel = (u8, u8, u8);
pub type Frame = [[Pixel; INTERNAL_WIDTH]; INTERNAL_HEIGHT];
