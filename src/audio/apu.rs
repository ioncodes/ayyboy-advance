use crate::memory::device::Addressable;

pub struct Apu {
    io: Box<[u8; (0x400008E - 0x4000080) + 1]>,
}

impl Apu {
    pub fn new() -> Apu {
        let io = Box::<[u8; (0x400008E - 0x4000080) + 1]>::new_zeroed();

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
            0x4000080..=0x400008E => self.io[(addr - 0x4000080) as usize],
            _ => unreachable!(),
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0x4000080..=0x400008E => self.io[(addr - 0x4000080) as usize] = value,
            _ => unreachable!(),
        }
    }
}
