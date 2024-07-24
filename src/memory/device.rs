pub trait IoDevice {
    fn read(&self, addr: u32) -> u16;
    fn write(&mut self, addr: u32, value: u16);
}
