use bitflags::bitflags;

use super::{FRAME_0_ADDRESS, FRAME_1_ADDRESS};

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
}
