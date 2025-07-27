use bitflags::{Flags, bitflags};
use tracing::warn;

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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum AddrControl {
    Increment,
    Decrement,
    Fixed,
    Reload,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum DmaTrigger {
    Immediate,
    VBlank,
    HBlank,
    Special,
}

impl DmaControl {
    pub fn dest_addr_control(&self) -> AddrControl {
        let value = (self.bits() & DmaControl::DEST_ADDR_CONTROL.bits()) >> 5;

        match value {
            0 => AddrControl::Increment,
            1 => AddrControl::Decrement,
            2 => AddrControl::Fixed,
            3 => AddrControl::Reload,
            _ => unreachable!(),
        }
    }

    pub fn src_addr_control(&self) -> AddrControl {
        let value = (self.bits() & DmaControl::SRC_ADDR_CONTROL.bits()) >> 7;

        match value {
            0 => AddrControl::Increment,
            1 => AddrControl::Decrement,
            2 => AddrControl::Fixed,
            3 => {
                warn!(target: "mmio", "DMA source address control set to Reload, this is not a valid state");
                AddrControl::Reload
            }
            _ => unreachable!(),
        }
    }

    pub fn is_repeat(&self) -> bool {
        self.contains(DmaControl::DMA_REPEAT)
    }

    pub fn transfer_size(&self) -> usize {
        if self.contains(DmaControl::DMA_TRANFER_TYPE) {
            4
        } else {
            2
        }
    }

    pub fn trigger(&self) -> DmaTrigger {
        let value = (self.bits() & DmaControl::START_TIMING.bits()) >> 12;

        match value {
            0 => DmaTrigger::Immediate,
            1 => DmaTrigger::VBlank,
            2 => DmaTrigger::HBlank,
            3 => DmaTrigger::Special,
            _ => unreachable!(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.contains(DmaControl::ENABLE)
    }

    pub fn trigger_irq(&self) -> bool {
        self.contains(DmaControl::IRQ_UPON_COMPLETE)
    }

    pub fn enable(&mut self) {
        self.insert(DmaControl::ENABLE);
    }

    pub fn disable(&mut self) {
        self.remove(DmaControl::ENABLE);
    }
}

bitflags! {
    #[derive(Default, PartialEq, Copy, Clone)]
    pub struct TimerControl: u16 {
        const PRESCALER_SELECTION = 0b0000_0000_0000_0011;
        const COUNT_UP_TIMING     = 0b0000_0000_0000_0100;
        const UNUSED0             = 0b0000_0000_0011_1000;
        const IRQ_ON_OVERFLOW     = 0b0000_0000_0100_0000;
        const ENABLE              = 0b0000_0000_1000_0000;
        const UNUSED1             = 0b1111_1111_0000_0000;
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

    pub fn set(&mut self, value: u32) {
        self.0 = (value & 0x000000FF) as u8;
        self.1 = ((value >> 8) & 0x000000FF) as u8;
        self.2 = ((value >> 16) & 0x000000FF) as u8;
        self.3 = ((value >> 24) & 0x000000FF) as u8;
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

    pub fn value_as_mut<T>(&mut self) -> &mut T
    where
        T: Flags<Bits = u16> + Copy,
    {
        unsafe { &mut *(self as *mut MappedRegister16 as *mut T) }
    }
}

impl Default for MappedRegister16 {
    fn default() -> Self {
        MappedRegister16(0, 0)
    }
}
