use super::device::Addressable;
use super::registers::{AddrControl, DmaControl, DmaTrigger, MappedRegister16, MappedRegister32};
use std::fmt::Display;

#[derive(Default, PartialEq, Clone, Copy)]
pub struct TransferChannel {
    pub src: MappedRegister32,
    pub dst: MappedRegister32,
    pub cnt: MappedRegister16,
    pub ctl: MappedRegister16,
    id: usize,
}

impl TransferChannel {
    pub fn new(id: usize) -> Self {
        TransferChannel {
            src: MappedRegister32::default(),
            dst: MappedRegister32::default(),
            cnt: MappedRegister16::default(),
            ctl: MappedRegister16::default(),
            id,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.ctl.value_as::<DmaControl>().is_enabled()
    }

    pub fn transfer_units(&self) -> u16 {
        let max_size = if self.id == 3 { 0xFFFF } else { 0x3FFF };
        let size = self.cnt.value() & max_size;
        if size == 0 {
            max_size
        } else {
            size
        }
    }

    pub fn transfer_size(&self) -> usize {
        self.ctl.value_as::<DmaControl>().transfer_size()
    }

    pub fn trigger(&self) -> DmaTrigger {
        self.ctl.value_as::<DmaControl>().trigger()
    }

    pub fn dst_addr_control(&self) -> AddrControl {
        self.cnt.value_as::<DmaControl>().dest_addr_control()
    }

    pub fn src_addr_control(&self) -> AddrControl {
        self.ctl.value_as::<DmaControl>().src_addr_control()
    }

    pub fn is_repeat(&self) -> bool {
        self.ctl.value_as::<DmaControl>().is_repeat()
    }

    pub fn enable(&mut self) {
        self.ctl.value_as_mut::<DmaControl>().enable();
    }

    pub fn disable(&mut self) {
        self.ctl.value_as_mut::<DmaControl>().disable();
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
pub struct Dma {
    pub channels: [TransferChannel; 4],
}

impl Dma {
    pub fn new() -> Self {
        Dma {
            channels: [
                TransferChannel::new(0),
                TransferChannel::new(1),
                TransferChannel::new(2),
                TransferChannel::new(3),
            ],
        }
    }
}

impl Addressable for Dma {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x040000B0..=0x040000B3 => self.channels[0].src.read(addr - 0x040000B0),
            0x040000B4..=0x040000B7 => self.channels[0].dst.read(addr - 0x040000B4),
            0x040000B8..=0x040000B9 => self.channels[0].cnt.read(addr - 0x040000B8),
            0x040000BA..=0x040000BB => self.channels[0].ctl.read(addr - 0x040000BA),
            0x040000BC..=0x040000BF => self.channels[1].src.read(addr - 0x040000BC),
            0x040000C0..=0x040000C3 => self.channels[1].dst.read(addr - 0x040000C0),
            0x040000C4..=0x040000C5 => self.channels[1].cnt.read(addr - 0x040000C4),
            0x040000C6..=0x040000C7 => self.channels[1].ctl.read(addr - 0x040000C6),
            0x040000C8..=0x040000CB => self.channels[2].src.read(addr - 0x040000C8),
            0x040000CC..=0x040000CF => self.channels[2].dst.read(addr - 0x040000CC),
            0x040000D0..=0x040000D1 => self.channels[2].cnt.read(addr - 0x040000D0),
            0x040000D2..=0x040000D3 => self.channels[2].ctl.read(addr - 0x040000D2),
            0x040000D4..=0x040000D7 => self.channels[3].src.read(addr - 0x040000D4),
            0x040000D8..=0x040000DB => self.channels[3].dst.read(addr - 0x040000D8),
            0x040000DC..=0x040000DD => self.channels[3].cnt.read(addr - 0x040000DC),
            0x040000DE..=0x040000DF => self.channels[3].ctl.read(addr - 0x040000DE),
            _ => panic!("Invalid DMA address: {:08x}", addr),
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0x040000B0..=0x040000B3 => self.channels[0].src.write(addr - 0x040000B0, value),
            0x040000B4..=0x040000B7 => self.channels[0].dst.write(addr - 0x040000B4, value),
            0x040000B8..=0x040000B9 => self.channels[0].cnt.write(addr - 0x040000B8, value),
            0x040000BA..=0x040000BB => self.channels[0].ctl.write(addr - 0x040000BA, value),
            0x040000BC..=0x040000BF => self.channels[1].src.write(addr - 0x040000BC, value),
            0x040000C0..=0x040000C3 => self.channels[1].dst.write(addr - 0x040000C0, value),
            0x040000C4..=0x040000C5 => self.channels[1].cnt.write(addr - 0x040000C4, value),
            0x040000C6..=0x040000C7 => self.channels[1].ctl.write(addr - 0x040000C6, value),
            0x040000C8..=0x040000CB => self.channels[2].src.write(addr - 0x040000C8, value),
            0x040000CC..=0x040000CF => self.channels[2].dst.write(addr - 0x040000CC, value),
            0x040000D0..=0x040000D1 => self.channels[2].cnt.write(addr - 0x040000D0, value),
            0x040000D2..=0x040000D3 => self.channels[2].ctl.write(addr - 0x040000D2, value),
            0x040000D4..=0x040000D7 => self.channels[3].src.write(addr - 0x040000D4, value),
            0x040000D8..=0x040000DB => self.channels[3].dst.write(addr - 0x040000D8, value),
            0x040000DC..=0x040000DD => self.channels[3].cnt.write(addr - 0x040000DC, value),
            0x040000DE..=0x040000DF => self.channels[3].ctl.write(addr - 0x040000DE, value),
            _ => panic!("Invalid DMA address: {:08x}", addr),
        }
    }
}

impl Display for Dma {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "DMA Channels:")?;
        for (i, channel) in self.channels.iter().enumerate() {
            write!(
                f,
                "\nChannel {} = src: {:08x}, dst: {:08x}, cnt: {:04x}, ctl: {:016b}",
                i,
                channel.src.value(),
                channel.dst.value(),
                channel.cnt.value(),
                channel.ctl.value()
            )?;
        }
        Ok(())
    }
}
