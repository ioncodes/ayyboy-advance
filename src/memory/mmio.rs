pub struct Mmio {
    pub ínternal_memory: Box<[u8; 0x04FFFFFF + 1]>,
    pub display_memory: Box<[u8; (0x07FFFFFF - 0x05000000) + 1]>,
    pub external_memory: Box<[u8; (0x0FFFFFFF - 0x08000000) + 1]>,
}

impl Mmio {
    pub fn new() -> Mmio {
        let internal_memory = Box::<[u8; 0x05000000]>::new_zeroed();
        let display_memory = Box::<[u8; 0x03000000]>::new_zeroed();
        let external_memory = Box::<[u8; 0x08000000]>::new_zeroed();

        Mmio {
            ínternal_memory: unsafe { internal_memory.assume_init() },
            display_memory: unsafe { display_memory.assume_init() },
            external_memory: unsafe { external_memory.assume_init() },
        }
    }

    pub fn read(&self, addr: u32) -> u8 {
        match addr {
            0x00000000..=0x04FFFFFF => self.ínternal_memory[(addr) as usize],
            0x05000000..=0x07FFFFFF => self.display_memory[(addr - 0x05000000) as usize],
            0x08000000..=0x0FFFFFFF => self.external_memory[(addr - 0x08000000) as usize],
            _ => panic!("Invalid memory address: {:08x}", addr),
        }
    }

    pub fn read_u16(&self, addr: u32) -> u16 {
        u16::from_le_bytes([self.read(addr), self.read(addr + 1)])
    }

    pub fn read_u32(&self, addr: u32) -> u32 {
        u32::from_le_bytes([
            self.read(addr),
            self.read(addr + 1),
            self.read(addr + 2),
            self.read(addr + 3),
        ])
    }

    pub fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0x00000000..=0x04FFFFFF => self.ínternal_memory[(addr) as usize] = value,
            0x05000000..=0x07FFFFFF => self.display_memory[(addr - 0x05000000) as usize] = value,
            0x08000000..=0x0FFFFFFF => self.external_memory[(addr - 0x08000000) as usize] = value,
            _ => panic!("Invalid memory address: {:08x}", addr),
        }
    }

    pub fn write_u16(&mut self, addr: u32, value: u16) {
        let [a, b] = value.to_le_bytes();
        self.write(addr, a);
        self.write(addr + 1, b);
    }

    pub fn write_u32(&mut self, addr: u32, value: u32) {
        let [a, b, c, d] = value.to_le_bytes();
        self.write(addr, a);
        self.write(addr + 1, b);
        self.write(addr + 2, c);
        self.write(addr + 3, d);
    }

    pub fn load(&mut self, addr: u32, data: &[u8]) {
        let addr = addr as usize;
        match addr {
            0x00000000..=0x04FFFFFF => {
                self.ínternal_memory[addr..addr + data.len()].copy_from_slice(data)
            }
            0x05000000..=0x07FFFFFF => self.display_memory
                [(addr - 0x05000000)..(addr - 0x05000000) + data.len()]
                .copy_from_slice(data),
            0x08000000..=0x0FFFFFFF => self.external_memory
                [(addr - 0x08000000)..(addr - 0x08000000) + data.len()]
                .copy_from_slice(data),
            _ => panic!("Invalid memory address: {:08x}", addr),
        }
    }
}
