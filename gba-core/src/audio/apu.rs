use crate::memory::device::Addressable;

pub struct Apu {
    io: Box<[u8; (0x040000A9 - 0x04000060) + 1]>,
}

impl Apu {
    pub fn new() -> Apu {
        let io = Box::<[u8; (0x040000A9 - 0x04000060) + 1]>::new_zeroed();

        Apu {
            io: unsafe { io.assume_init() },
        }
    }
}

impl Addressable for Apu {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            // SOUNDBIAS register
            0x04000088 => 0x00,
            0x04000089 => 0x02,
            // rest of the registers
            _ => self.io[(addr - 0x4000060) as usize],
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        match addr {
            _ => self.io[(addr - 0x4000060) as usize] = value,
        }
    }
}
