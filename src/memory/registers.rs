use bitflags::{bitflags, Flags};

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

bitflags! {
    #[derive(Default, Copy, Clone)]
    pub struct DmaControl: u16 {
        const UNUSED            = 0b0000_0000_0001_1111;
        const DEST_ADDR_CONTROL = 0b0000_0000_0110_0000;
        const SRC_ADDR_CONTROL  = 0b0000_0001_1000_0000;
        const DMA_REPEAT        = 0b0000_0010_0000_0000;
        const DMA_TRANFER_TYPE  = 0b0000_0100_0000_0000;
        const GAMEPAK_DRQ       = 0b0000_1000_0000_0000;
        const START_TIMING      = 0b0011_0000_0000_0000;
        const IRQ_UPON_COMPLETE = 0b0100_0000_0000_0000;
        const ENABLE            = 0b1000_0000_0000_0000;
    }
}

#[derive(PartialEq, Clone, Copy)]
pub struct MappedRegister32(u8, u8, u8, u8);

impl MappedRegister32 {
    pub fn read(&self, addr: u32) -> u8 {
        match addr {
            0 => self.0,
            1 => self.1,
            2 => self.2,
            3 => self.3,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0 => self.0 = value,
            1 => self.1 = value,
            2 => self.2 = value,
            3 => self.3 = value,
            _ => unreachable!(),
        }
    }

    pub fn value(&self) -> u32 {
        (self.0 as u32) | ((self.1 as u32) << 8) | ((self.2 as u32) << 16) | ((self.3 as u32) << 24)
    }
}

impl Default for MappedRegister32 {
    fn default() -> Self {
        MappedRegister32(0, 0, 0, 0)
    }
}

#[derive(PartialEq, Clone, Copy)]
pub struct MappedRegister16(u8, u8);

impl MappedRegister16 {
    pub fn read(&self, addr: u32) -> u8 {
        match addr {
            0 => self.0,
            1 => self.1,
            _ => unreachable!(),
        }
    }

    pub fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0 => self.0 = value,
            1 => self.1 = value,
            _ => unreachable!(),
        }
    }

    pub fn set(&mut self, value: u16) {
        self.0 = (value & 0x00FF) as u8;
        self.1 = ((value >> 8) & 0x00FF) as u8;
    }

    pub fn value(&self) -> u16 {
        (self.0 as u16) | ((self.1 as u16) << 8)
    }

    pub fn value_as<T>(&self) -> T
    where
        T: Flags<Bits = u16> + Copy,
    {
        T::from_bits_truncate(self.value())
    }
}

impl Default for MappedRegister16 {
    fn default() -> Self {
        MappedRegister16(0, 0)
    }
}
