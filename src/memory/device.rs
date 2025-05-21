use bitflags::Flags;
use spdlog::trace;

#[allow(dead_code)]
pub trait Addressable {
    fn read(&self, addr: u32) -> u8;
    fn write(&mut self, addr: u32, value: u8);

    fn load(&mut self, addr: u32, data: &[u8]) {
        for (i, &byte) in data.iter().enumerate() {
            self.write(addr + i as u32, byte);
        }
    }

    fn read_u16(&self, addr: u32) -> u16 {
        u16::from_le_bytes([self.read(addr), self.read(addr + 1)])
    }

    fn read_u32(&self, addr: u32) -> u32 {
        u32::from_le_bytes([
            self.read(addr),
            self.read(addr + 1),
            self.read(addr + 2),
            self.read(addr + 3),
        ])
    }

    fn write_u16(&mut self, addr: u32, value: u16) {
        let [a, b] = value.to_le_bytes();
        self.write(addr, a);
        self.write(addr + 1, b);
    }

    fn write_u32(&mut self, addr: u32, value: u32) {
        let [a, b, c, d] = value.to_le_bytes();
        self.write(addr, a);
        self.write(addr + 1, b);
        self.write(addr + 2, c);
        self.write(addr + 3, d);
    }

    fn read_as<T: Flags<Bits = u16>>(&self, addr: u32) -> T {
        T::from_bits_truncate(self.read_u16(addr))
    }
}

pub struct IoRegister<T = u16>(pub T);

impl<T> IoRegister<T> {
    pub fn set(&mut self, value: T)
    where
        T: Copy,
    {
        self.0 = value;
    }

    pub fn value(&self) -> &T {
        &self.0
    }
}

impl IoRegister<u16> {
    pub fn write_high(&mut self, value: u8) {
        self.0 = (self.0 & 0x00ff) | ((value as u16) << 8);
    }

    pub fn write_low(&mut self, value: u8) {
        self.0 = (self.0 & 0xff00) | (value as u16);
    }

    pub fn write(&mut self, addr: u32, value: u8) {
        if addr % 2 == 0 {
            self.write_low(value);
        } else {
            self.write_high(value);
        }
    }

    pub fn read_high(&self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub fn read_low(&self) -> u8 {
        self.0 as u8
    }

    pub fn read(&self, addr: u32) -> u8 {
        if addr % 2 == 0 {
            self.read_low()
        } else {
            self.read_high()
        }
    }
}

impl<T> IoRegister<T>
where
    T: Flags<Bits = u16> + Copy,
{
    pub fn set_flags(&mut self, flags: T) {
        self.0 = self.0.union(flags);
    }

    pub fn clear_flags(&mut self, flags: T) {
        self.0 = self.0.difference(flags);
    }

    pub fn toggle_flags(&mut self, flags: T) {
        self.0 = self.0.symmetric_difference(flags);
    }

    pub fn contains_flags(&self, flags: T) -> bool {
        self.0.contains(flags)
    }
}

impl<T> Addressable for IoRegister<T>
where
    T: Flags<Bits = u16> + Copy,
{
    fn read(&self, addr: u32) -> u8 {
        if addr % 2 == 0 {
            self.0.bits() as u8
        } else {
            (self.0.bits() >> 8) as u8
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        // Interrupts must be manually acknowledged by writing a "1" to one of the IRQ bits, the IRQ bit will then be cleared.
        match addr {
            0x4000202 => {
                let ack = value as u16;
                self.0 = T::from_bits_truncate(self.0.bits() & !ack);
                trace!("Acknowledged interrupt, IF now: {:04x}", self.0.bits());
                return;
            }
            0x4000203 => {
                let ack = (value as u16) << 8;
                self.0 = T::from_bits_truncate(self.0.bits() & !ack);
                trace!("Acknowledged interrupt, IF now: {:04x}", self.0.bits());
                return;
            }
            _ => {}
        }

        if addr % 2 == 0 {
            self.0 = T::from_bits_truncate((self.0.bits() & 0xff00) | (value as u16));
        } else {
            self.0 = T::from_bits_truncate((self.0.bits() & 0x00ff) | ((value as u16) << 8));
        }
    }
}

impl<T: Default> Default for IoRegister<T> {
    fn default() -> Self {
        IoRegister(T::default())
    }
}
