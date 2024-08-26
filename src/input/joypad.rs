use super::registers::{KeyControl, KeyInput};
use crate::memory::device::Addressable;

pub struct Joypad {
    status: KeyInput,
    irq_control: KeyControl,
}

impl Joypad {
    pub fn new() -> Joypad {
        Joypad {
            status: KeyInput::empty(),
            irq_control: KeyControl::empty(),
        }
    }

    pub fn set_key_state(&mut self, key: KeyInput, pressed: bool) {
        if pressed {
            self.status.remove(key);
        } else {
            self.status.insert(key);
        }
    }
}

impl Addressable for Joypad {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x04000130 => self.status.bits() as u8,
            0x04000131 => ((self.status.bits() & 0xff00) >> 8) as u8,
            0x04000132 => self.irq_control.bits() as u8,
            0x04000133 => ((self.irq_control.bits() & 0xff00) >> 8) as u8,
            _ => unreachable!(),
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0x04000130 => {
                self.status = KeyInput::from_bits_truncate(u16::from_le_bytes([value, self.status.bits() as u8]));
            }
            0x04000131 => {
                self.status = KeyInput::from_bits_truncate(u16::from_le_bytes([
                    ((self.status.bits() & 0xff00) >> 8) as u8,
                    value,
                ]));
            }
            _ => unreachable!(),
        }
    }
}
