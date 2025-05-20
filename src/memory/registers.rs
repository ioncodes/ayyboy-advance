use bitflags::bitflags;

bitflags! {
    #[derive(Default, Copy, Clone)]
    pub struct Interrupt: u16 {
        const VBLANK = 1 << 0;
        const HBLANK = 1 << 1;
        const VCOUNT = 1 << 2;
        const TIMER0 = 1 << 3;
        const TIMER1 = 1 << 4;
        const TIMER2 = 1 << 5;
        const TIMER3 = 1 << 6;
        const SERIAL = 1 << 7;
        const DMA0 = 1 << 8;
        const DMA1 = 1 << 9;
        const DMA2 = 1 << 10;
        const DMA3 = 1 << 11;
        const KEYPAD = 1 << 12;
        const GAMEPAK = 1 << 13;
        const UNUSED0 = 1 << 14;
        const UNUSED1 = 1 << 15;
    }
}
