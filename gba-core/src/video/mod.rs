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

pub type Rgb = (u8, u8, u8);
pub type Frame = [[Rgb; SCREEN_WIDTH]; SCREEN_HEIGHT];
