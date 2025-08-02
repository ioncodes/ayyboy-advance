use super::registers::{KeyControl, KeyInput};
use crate::memory::device::Addressable;

pub struct Joypad {
    status: KeyInput,
    irq_control: KeyControl,
}

impl Joypad {
    pub fn new() -> Joypad {
        Joypad {
            status: KeyInput::all(),
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

    pub fn is_key_pressed(&self, key: KeyInput) -> bool {
        !self.status.contains(key)
    }

    pub fn check_keypad_interrupt(&self) -> bool {
        if !self.irq_control.contains(KeyControl::IRQ_ENABLE) {
            return false;
        }

        // Get the keys to check for interrupt
        let keys_to_check = KeyInput::from_bits_truncate(self.irq_control.bits() & 0x03FF);

        if self.irq_control.contains(KeyControl::IRQ_CONDITION) {
            // ALL specified keys must be pressed
            let pressed_keys = KeyInput::all() ^ self.status; // Invert to get pressed keys
            (pressed_keys & keys_to_check) == keys_to_check
        } else {
            // ANY specified key must be pressed
            let pressed_keys = KeyInput::all() ^ self.status; // Invert to get pressed keys
            !(pressed_keys & keys_to_check).is_empty()
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
            0x04000132 => {
                self.irq_control =
                    KeyControl::from_bits_truncate(u16::from_le_bytes([value, self.irq_control.bits() as u8]));
            }
            0x04000133 => {
                self.irq_control = KeyControl::from_bits_truncate(u16::from_le_bytes([
                    ((self.irq_control.bits() & 0xff00) >> 8) as u8,
                    value,
                ]));
            }
            _ => unreachable!(),
        }
    }
}
