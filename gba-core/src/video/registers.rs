use super::{FRAME_0_ADDRESS, FRAME_1_ADDRESS, TILEMAP_ENTRY_SIZE, TILESET_ENTRY_SIZE};
use bitflags::bitflags;
use tracing::warn;

bitflags! {
    #[derive(Default, Copy, Clone)]
    pub struct DispStat: u16 {
        const V_COUNT_SETTING   = 0b1111_1111_0000_0000;
        const V_COUNTER_ENABLE  = 1 << 5;
        const HBLANK_IRQ_ENABLE = 1 << 4;
        const VBLANK_IRQ_ENABLE = 1 << 3;
        const VCOUNTER_FLAG     = 1 << 2;
        const HBLANK_FLAG       = 1 << 1;
        const VBLANK_FLAG       = 1 << 0;
    }

    #[derive(Default, Copy, Clone)]
    pub struct DispCnt: u16 {
        const BG_MODE               = 0b0000_0000_0000_0111;
        const CGB_MODE              = 1 << 3;
        const DISPLAY_FRAME_SELECT  = 1 << 4;
        const HBLANK_INTERVAL_FREE  = 1 << 5;
        const OBJ_CHAR_MAPPING      = 1 << 6;
        const FORCED_BLANK          = 1 << 7;
        const BG0_ON                = 1 << 8;
        const BG1_ON                = 1 << 9;
        const BG2_ON                = 1 << 10;
        const BG3_ON                = 1 << 11;
        const OBJ_ON                = 1 << 12;
        const WIN0_ON               = 1 << 13;
        const WIN1_ON               = 1 << 14;
        const OBJ_WIN_ON            = 1 << 15;
    }
}

bitflags! {
    #[derive(Default, Copy, Clone)]
    pub struct BgCnt: u16 {
        const BG_PRIORITY         = 0b0000_0000_0000_0011;
        const CHAR_BASE_ADDR      = 0b0000_0000_0000_1100;
        const UNUSED0             = 0b0000_0000_0011_0000;
        const MOSAIC              = 0b0000_0000_0100_0000;
        const COLOR_256           = 0b0000_0000_1000_0000;
        const SCREEN_BASE_ADDR    = 0b0001_1111_0000_0000;
        const DISPLAY_OVERFLOW    = 0b0010_0000_0000_0000;
        const SCREEN_SIZE         = 0b1100_0000_0000_0000;
    }
}

#[derive(PartialEq)]
pub enum Dimension {
    OneDimensional,
    TwoDimensional,
}

impl std::fmt::Display for Dimension {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Dimension::OneDimensional => write!(f, "1D"),
            Dimension::TwoDimensional => write!(f, "2D"),
        }
    }
}

impl DispCnt {
    pub fn bg_mode(&self) -> u8 {
        (self.bits() & DispCnt::BG_MODE.bits()) as u8
    }

    pub fn frame_address(&self) -> u32 {
        if !self.contains(DispCnt::DISPLAY_FRAME_SELECT) {
            FRAME_0_ADDRESS
        } else {
            FRAME_1_ADDRESS
        }
    }

    pub fn dimension(&self) -> Dimension {
        if self.contains(DispCnt::OBJ_CHAR_MAPPING) {
            Dimension::OneDimensional
        } else {
            Dimension::TwoDimensional
        }
    }
}

#[derive(Copy, Clone)]
pub enum InternalScreenSize {
    Text256x256,
    Text512x256,
    Text256x512,
    Text512x512,
    Affine128x128,
    Affine256x256,
    Affine512x512,
    Affine1024x1024,
}

impl InternalScreenSize {
    pub fn width(&self) -> usize {
        match self {
            InternalScreenSize::Text256x256 => 256,
            InternalScreenSize::Text512x256 => 512,
            InternalScreenSize::Text256x512 => 256,
            InternalScreenSize::Text512x512 => 512,
            InternalScreenSize::Affine128x128 => 128,
            InternalScreenSize::Affine256x256 => 256,
            InternalScreenSize::Affine512x512 => 512,
            InternalScreenSize::Affine1024x1024 => 1024,
        }
    }

    pub fn height(&self) -> usize {
        match self {
            InternalScreenSize::Text256x256 => 256,
            InternalScreenSize::Text512x256 => 256,
            InternalScreenSize::Text256x512 => 512,
            InternalScreenSize::Text512x512 => 512,
            InternalScreenSize::Affine128x128 => 128,
            InternalScreenSize::Affine256x256 => 256,
            InternalScreenSize::Affine512x512 => 512,
            InternalScreenSize::Affine1024x1024 => 1024,
        }
    }
}

impl std::fmt::Display for InternalScreenSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InternalScreenSize::Text256x256 => write!(f, "256x256 (Text)"),
            InternalScreenSize::Text512x256 => write!(f, "512x256 (Text)"),
            InternalScreenSize::Text256x512 => write!(f, "256x512 (Text)"),
            InternalScreenSize::Text512x512 => write!(f, "512x512 (Text)"),
            InternalScreenSize::Affine128x128 => write!(f, "128x128 (Affine)"),
            InternalScreenSize::Affine256x256 => write!(f, "256x256 (Affine)"),
            InternalScreenSize::Affine512x512 => write!(f, "512x512 (Affine)"),
            InternalScreenSize::Affine1024x1024 => write!(f, "1024x1024 (Affine)"),
        }
    }
}

#[derive(PartialEq)]
pub enum ColorDepth {
    Bpp4,
    Bpp8,
}

impl std::fmt::Display for ColorDepth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColorDepth::Bpp4 => write!(f, "4bpp"),
            ColorDepth::Bpp8 => write!(f, "8bpp"),
        }
    }
}

impl BgCnt {
    pub fn screen_size(&self, bg: usize, bg_mode: u8) -> InternalScreenSize {
        match (bg_mode, (*self & BgCnt::SCREEN_SIZE).bits()) {
            (0, 0b0000_0000_0000_0000) => InternalScreenSize::Text256x256,
            (0, 0b0100_0000_0000_0000) => InternalScreenSize::Text512x256,
            (0, 0b1000_0000_0000_0000) => InternalScreenSize::Text256x512,
            (0, 0b1100_0000_0000_0000) => InternalScreenSize::Text512x512,
            (1, 0b0000_0000_0000_0000) if bg == 2 => InternalScreenSize::Affine128x128,
            (1, 0b0100_0000_0000_0000) if bg == 2 => InternalScreenSize::Affine256x256,
            (1, 0b1000_0000_0000_0000) if bg == 2 => InternalScreenSize::Affine512x512,
            (1, 0b1100_0000_0000_0000) if bg == 2 => InternalScreenSize::Affine1024x1024,
            (1, 0b0000_0000_0000_0000) => InternalScreenSize::Text256x256,
            (1, 0b0100_0000_0000_0000) => InternalScreenSize::Text512x256,
            (1, 0b1000_0000_0000_0000) => InternalScreenSize::Text256x512,
            (1, 0b1100_0000_0000_0000) => InternalScreenSize::Text512x512,
            (_, 0b0000_0000_0000_0000) => InternalScreenSize::Affine128x128,
            (_, 0b0100_0000_0000_0000) => InternalScreenSize::Affine256x256,
            (_, 0b1000_0000_0000_0000) => InternalScreenSize::Affine512x512,
            (_, 0b1100_0000_0000_0000) => InternalScreenSize::Affine1024x1024,
            _ => unreachable!(),
        }
    }

    pub fn tileset_addr(&self) -> u32 {
        let addr = ((*self & BgCnt::CHAR_BASE_ADDR).bits() >> 2) as u32;
        0x6000000 + (addr * TILESET_ENTRY_SIZE as u32)
    }

    pub fn tilemap_addr(&self) -> u32 {
        let addr = ((*self & BgCnt::SCREEN_BASE_ADDR).bits() >> 8) as u32;
        0x6000000 + (addr * TILEMAP_ENTRY_SIZE as u32)
    }

    pub fn bpp(&self) -> ColorDepth {
        if self.contains(BgCnt::COLOR_256) {
            ColorDepth::Bpp8
        } else {
            ColorDepth::Bpp4
        }
    }

    pub fn priority(&self) -> usize {
        (self.bits() & BgCnt::BG_PRIORITY.bits()) as usize
    }
}

bitflags! {
    #[derive(Default, Copy, Clone)]
    pub struct BgOffset: u16 {
        const OFFSET = 0b0000_0001_1111_1111;
        const UNUSED = 0b1111_1110_0000_0000;
    }
}

impl BgOffset {
    pub fn offset(&self) -> usize {
        (self.bits() & BgOffset::OFFSET.bits()) as usize
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ObjShape {
    Square,
    Horizontal,
    Vertical,
}

bitflags! {
    #[derive(Default, Copy, Clone)]
    pub struct ObjAttribute0: u16 {
        const Y_COORDINATE          = 0b0000_0000_1111_1111;
        const ROTATION_SCALING      = 0b0000_0001_0000_0000;
        const DISABLE_OR_DBL_SIZE   = 0b0000_0010_0000_0000;
        const OBJ_MODE              = 0b0000_1100_0000_0000;
        const OBJ_MOSAIC            = 0b0001_0000_0000_0000;
        const COLOR_256             = 0b0010_0000_0000_0000;
        const SHAPE                 = 0b1100_0000_0000_0000;
    }
}

impl ObjAttribute0 {
    pub fn y_coordinate(&self) -> usize {
        (self.bits() & ObjAttribute0::Y_COORDINATE.bits()) as usize
    }

    pub fn shape(&self) -> ObjShape {
        match (*self & ObjAttribute0::SHAPE).bits() {
            0b0000_0000_0000_0000 => ObjShape::Square,
            0b0100_0000_0000_0000 => ObjShape::Horizontal,
            0b1000_0000_0000_0000 => ObjShape::Vertical,
            _ => unreachable!("prohibited value"),
        }
    }

    pub fn disabled(&self) -> bool {
        if self.contains(ObjAttribute0::ROTATION_SCALING) {
            warn!(target: "ppu", "DISABLE flag cannot be used with rotation/scaling");
        }

        self.contains(ObjAttribute0::DISABLE_OR_DBL_SIZE)
    }

    pub fn bpp(&self) -> ColorDepth {
        if self.contains(ObjAttribute0::COLOR_256) {
            ColorDepth::Bpp8
        } else {
            ColorDepth::Bpp4
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum ObjSize {
    Square8x8,
    Square16x16,
    Square32x32,
    Square64x64,
    Horizontal16x8,
    Horizontal32x8,
    Horizontal32x16,
    Horizontal64x32,
    Vertical8x16,
    Vertical8x32,
    Vertical16x32,
    Vertical32x64,
}

impl std::fmt::Display for ObjSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObjSize::Square8x8 => write!(f, "8x8"),
            ObjSize::Square16x16 => write!(f, "16x16"),
            ObjSize::Square32x32 => write!(f, "32x32"),
            ObjSize::Square64x64 => write!(f, "64x64"),
            ObjSize::Horizontal16x8 => write!(f, "16x8"),
            ObjSize::Horizontal32x8 => write!(f, "32x8"),
            ObjSize::Horizontal32x16 => write!(f, "32x16"),
            ObjSize::Horizontal64x32 => write!(f, "64x32"),
            ObjSize::Vertical8x16 => write!(f, "8x16"),
            ObjSize::Vertical8x32 => write!(f, "8x32"),
            ObjSize::Vertical16x32 => write!(f, "16x32"),
            ObjSize::Vertical32x64 => write!(f, "32x64"),
        }
    }
}

bitflags! {
    #[derive(Default, Copy, Clone)]
    pub struct ObjAttribute1: u16 {
        const X_COORDINATE  = 0b0000_0001_1111_1111;
        const UNUSED        = 0b0000_1110_0000_0000;
        const X_FLIP        = 0b0001_0000_0000_0000;
        const Y_FLIP        = 0b0010_0000_0000_0000;
        const OBJ_SIZE      = 0b1100_0000_0000_0000;
    }
}

impl ObjAttribute1 {
    pub fn x_coordinate(&self) -> usize {
        (self.bits() & ObjAttribute1::X_COORDINATE.bits()) as usize
    }

    pub fn x_flip(&self) -> bool {
        self.contains(ObjAttribute1::X_FLIP)
    }

    pub fn y_flip(&self) -> bool {
        self.contains(ObjAttribute1::Y_FLIP)
    }

    pub fn size(&self, shape: ObjShape) -> ObjSize {
        match (*self & ObjAttribute1::OBJ_SIZE).bits() {
            0b0000_0000_0000_0000 if shape == ObjShape::Square => ObjSize::Square8x8,
            0b0100_0000_0000_0000 if shape == ObjShape::Square => ObjSize::Square16x16,
            0b1000_0000_0000_0000 if shape == ObjShape::Square => ObjSize::Square32x32,
            0b1100_0000_0000_0000 if shape == ObjShape::Square => ObjSize::Square64x64,
            0b0000_0000_0000_0000 if shape == ObjShape::Horizontal => ObjSize::Horizontal16x8,
            0b0100_0000_0000_0000 if shape == ObjShape::Horizontal => ObjSize::Horizontal32x8,
            0b1000_0000_0000_0000 if shape == ObjShape::Horizontal => ObjSize::Horizontal32x16,
            0b1100_0000_0000_0000 if shape == ObjShape::Horizontal => ObjSize::Horizontal64x32,
            0b0000_0000_0000_0000 if shape == ObjShape::Vertical => ObjSize::Vertical8x16,
            0b0100_0000_0000_0000 if shape == ObjShape::Vertical => ObjSize::Vertical8x32,
            0b1000_0000_0000_0000 if shape == ObjShape::Vertical => ObjSize::Vertical16x32,
            0b1100_0000_0000_0000 if shape == ObjShape::Vertical => ObjSize::Vertical32x64,
            _ => unreachable!("Invalid OBJ_SIZE bits"),
        }
    }
}

bitflags! {
    #[derive(Default, Copy, Clone)]
    pub struct ObjAttribute2: u16 {
        const TILE_NUMBER = 0b0000_0011_1111_1111;
        const PRIORITY    = 0b0000_1100_0000_0000;
        const PALLETE     = 0b1111_0000_0000_0000;
    }
}

impl ObjAttribute2 {
    pub fn tile_number(&self) -> usize {
        (self.bits() & ObjAttribute2::TILE_NUMBER.bits()) as usize
    }

    pub fn priority(&self) -> usize {
        ((self.bits() & ObjAttribute2::PRIORITY.bits()) >> 10) as usize
    }

    pub fn palette(&self) -> usize {
        ((self.bits() & ObjAttribute2::PALLETE.bits()) >> 12) as usize
    }
}
