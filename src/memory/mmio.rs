pub struct Mmio {
    pub memory: Box<[u8; 0x10000000]>,
}

impl Mmio {
    pub fn new() -> Mmio {
        let memory = Box::<[u8; 0x10000000]>::new_zeroed();

        Mmio {
            memory: unsafe { memory.assume_init() },
        }
    }

    pub fn read(&self, addr: u32) -> u8 {
        self.memory[addr as usize]
    }

    pub fn read_u16(&self, addr: u32) -> u16 {
        u16::from_le_bytes([self.memory[addr as usize], self.memory[(addr + 1) as usize]])
    }

    pub fn read_u32(&self, addr: u32) -> u32 {
        u32::from_le_bytes([
            self.memory[addr as usize],
            self.memory[(addr + 1) as usize],
            self.memory[(addr + 2) as usize],
            self.memory[(addr + 3) as usize],
        ])
    }

    pub fn write(&mut self, addr: u32, value: u8) {
        self.memory[addr as usize] = value;
    }

    pub fn write_u16(&mut self, addr: u32, value: u16) {
        let [a, b] = value.to_le_bytes();
        self.memory[addr as usize] = a;
        self.memory[(addr + 1) as usize] = b;
    }

    pub fn write_u32(&mut self, addr: u32, value: u32) {
        let [a, b, c, d] = value.to_le_bytes();
        self.memory[addr as usize] = a;
        self.memory[(addr + 1) as usize] = b;
        self.memory[(addr + 2) as usize] = c;
        self.memory[(addr + 3) as usize] = d;
    }

    pub fn load(&mut self, addr: u32, data: &[u8]) {
        let addr = addr as usize;
        self.memory[addr..(addr + data.len())].copy_from_slice(data);
    }
}
