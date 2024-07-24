use bitflags::bitflags;

bitflags! {
    pub struct DispStat: u16 {
        const V_COUNT_SETTING   = 0b1111_1111_0000_0000;
        const V_COUNTER_ENABLE  = 1 << 5;
        const HBLANK_IRQ_ENABLE = 1 << 4;
        const VBLANK_IRQ_ENABLE = 1 << 3;
        const VCOUNTER_FLAG     = 1 << 2;
        const HBLANK_FLAG       = 1 << 1;
        const VBLANK_FLAG       = 1 << 0;
    }
}
