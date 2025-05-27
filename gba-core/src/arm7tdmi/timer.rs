use crate::memory::device::{Addressable, IoRegister};
use crate::memory::registers::TimerControl;

#[derive(Default, PartialEq, Clone, Copy)]
pub struct Timer {
    pub counter: IoRegister<u16>,
    pub reload: IoRegister<u16>,
    pub control: IoRegister<TimerControl>,
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            counter: IoRegister::default(),
            reload: IoRegister::default(),
            control: IoRegister::default(),
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.control.contains_flags(TimerControl::ENABLE)
    }

    pub fn tick(&mut self) {
        self.counter.set(self.counter.value().wrapping_add(1));

        if self.counter.0 == 0 {
            self.counter.set(self.reload.0);
        }
    }
}

#[derive(Default, PartialEq, Clone, Copy)]
pub struct Timers {
    pub timers: [Timer; 4],
}

impl Timers {
    pub fn new() -> Self {
        Timers {
            timers: [Timer::new(), Timer::new(), Timer::new(), Timer::new()],
        }
    }

    pub fn tick(&mut self) {
        for timer in &mut self.timers {
            if timer.is_enabled() {
                timer.tick();
            }
        }
    }
}

impl Addressable for Timers {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x04000100..=0x04000101 => self.timers[0].counter.read(addr),
            0x04000102..=0x04000103 => self.timers[0].control.read(addr),
            0x04000104..=0x04000105 => self.timers[1].counter.read(addr),
            0x04000106..=0x04000107 => self.timers[1].control.read(addr),
            0x04000108..=0x04000109 => self.timers[2].counter.read(addr),
            0x0400010A..=0x0400010B => self.timers[2].control.read(addr),
            0x0400010C..=0x0400010D => self.timers[3].counter.read(addr),
            0x0400010E..=0x0400010F => self.timers[3].control.read(addr),
            _ => unreachable!(),
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0x04000100..=0x04000101 => self.timers[0].reload.write(addr, value),
            0x04000102..=0x04000103 => self.timers[0].control.write(addr, value),
            0x04000104..=0x04000105 => self.timers[1].reload.write(addr, value),
            0x04000106..=0x04000107 => self.timers[1].control.write(addr, value),
            0x04000108..=0x04000109 => self.timers[2].reload.write(addr, value),
            0x0400010A..=0x0400010B => self.timers[2].control.write(addr, value),
            0x0400010C..=0x0400010D => self.timers[3].reload.write(addr, value),
            0x0400010E..=0x0400010F => self.timers[3].control.write(addr, value),
            _ => unreachable!(),
        }
    }
}
