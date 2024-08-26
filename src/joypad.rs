use log::error;

use crate::memory::device::Addressable;

pub struct Joypad {}

impl Joypad {
    pub fn new() -> Joypad {
        Joypad {}
    }
}

impl Addressable for Joypad {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x04000130..=0x04000131 => {
                error!("Read from KEYINPUT");
                !0
            }
            0x04000132..=0x04000133 => {
                error!("Read from KEYCNT");
                0
            }
            _ => unreachable!(),
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0x04000130..=0x04000131 => error!("Write to KEYINPUT: {:02x}", value),
            0x04000132..=0x04000133 => error!("Write to KEYCNT: {:02x}", value),
            _ => unreachable!(),
        }
    }
}
