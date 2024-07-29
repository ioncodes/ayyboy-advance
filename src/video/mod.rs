pub mod ppu;
mod registers;

pub const SCREEN_WIDTH: usize = 240;
pub const SCREEN_HEIGHT: usize = 160;

pub const DISPCNT_ADDR: u32 = 0x04000000;
pub const DISPSTAT_ADDR: u32 = 0x04000004;

pub type Pixel = (u8, u8, u8);
pub type Frame = [[Pixel; SCREEN_WIDTH]; SCREEN_HEIGHT];
