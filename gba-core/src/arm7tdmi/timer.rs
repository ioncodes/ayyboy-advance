use crate::memory::device::{Addressable, IoRegister};
use crate::memory::registers::TimerControl;

#[derive(Default, PartialEq, Clone, Copy)]
pub struct Timer {
    pub counter: IoRegister<u16>,
    pub reload: IoRegister<u16>,
    pub control: IoRegister<TimerControl>,
    pub prescaler_counter: u32, // Track cycles for prescaler
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            counter: IoRegister::default(),
            reload: IoRegister::default(),
            control: IoRegister::default(),
            prescaler_counter: 0,
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.control.contains_flags(TimerControl::ENABLE)
    }

    pub fn tick(&mut self) -> bool {
        let old_counter = *self.counter.value();
        self.counter.set(old_counter.wrapping_add(1));

        // Check for overflow (counter wrapped from 0xFFFF to 0x0000)
        let overflowed = old_counter == 0xFFFF;

        if overflowed {
            self.counter.set(*self.reload.value());
        }

        // Return true if overflow occurred and IRQ is enabled
        overflowed && self.control.contains_flags(TimerControl::IRQ_ON_OVERFLOW)
    }

    pub fn get_prescaler_cycles(&self) -> u32 {
        let prescaler_bits = (*self.control.value() & TimerControl::PRESCALER_SELECTION).bits();
        match prescaler_bits {
            0 => 1,    // F/1 (16.78 MHz)
            1 => 64,   // F/64 (262.21 KHz)
            2 => 256,  // F/256 (65.536 KHz)
            3 => 1024, // F/1024 (16.384 KHz)
            _ => unreachable!(),
        }
    }

    pub fn tick_cycles(&mut self, cycles: u32) -> bool {
        if !self.is_enabled() {
            return false;
        }

        // If cascade mode (count-up timing), don't increment based on cycles
        if self.control.contains_flags(TimerControl::COUNT_UP_TIMING) {
            return false;
        }

        let prescaler_cycles = self.get_prescaler_cycles();
        self.prescaler_counter += cycles;

        let mut overflow = false;
        while self.prescaler_counter >= prescaler_cycles {
            self.prescaler_counter -= prescaler_cycles;

            let old_counter = *self.counter.value();
            let new_counter = old_counter.wrapping_add(1);
            self.counter.set(new_counter);

            // Check for overflow (counter wrapped from 0xFFFF to 0x0000)
            let overflowed = old_counter == 0xFFFF;

            if overflowed {
                self.counter.set(*self.reload.value());
                overflow = true;
            }
        }

        // Return true if overflow occurred and IRQ is enabled
        overflow && self.control.contains_flags(TimerControl::IRQ_ON_OVERFLOW)
    }

    pub fn cascade_increment(&mut self) -> bool {
        if !self.is_enabled() {
            return false;
        }

        let old_counter = *self.counter.value();
        let new_counter = old_counter.wrapping_add(1);
        self.counter.set(new_counter);

        // Check for overflow (counter wrapped from 0xFFFF to 0x0000)
        let overflowed = old_counter == 0xFFFF;

        if overflowed {
            self.counter.set(*self.reload.value());
        }

        // Return true if overflow occurred and IRQ is enabled
        overflowed && self.control.contains_flags(TimerControl::IRQ_ON_OVERFLOW)
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

    pub fn tick(&mut self) -> [bool; 4] {
        let mut timer_irqs = [false; 4];

        for (i, timer) in self.timers.iter_mut().enumerate() {
            if timer.is_enabled() {
                timer_irqs[i] = timer.tick();
            }
        }

        timer_irqs
    }

    pub fn tick_cycles(&mut self, cycles: u32) -> [bool; 4] {
        let mut overflows = [false; 4];

        // Timer 0 - always based on cycles
        overflows[0] = self.timers[0].tick_cycles(cycles);

        // Timers 1-3 can cascade from previous timer
        for i in 1..4 {
            if self.timers[i].control.contains_flags(TimerControl::COUNT_UP_TIMING) {
                // Cascade mode: increment when previous timer overflows
                if overflows[i - 1] {
                    overflows[i] = self.timers[i].cascade_increment();
                }
            } else {
                // Normal mode: increment based on cycles
                overflows[i] = self.timers[i].tick_cycles(cycles);
            }
        }

        overflows
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
