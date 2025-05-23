pub mod ppu;
pub mod registers;

pub const SCREEN_WIDTH: usize = 240;
pub const SCREEN_HEIGHT: usize = 160;

pub const FRAME_0_ADDRESS: u32 = 0x0600_0000;
pub const FRAME_1_ADDRESS: u32 = 0x0600_A000;

pub type Pixel = (u8, u8, u8);
pub type Frame = [[Pixel; SCREEN_WIDTH]; SCREEN_HEIGHT];
