use bitflags::Flags;

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
