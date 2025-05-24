use crate::arm7tdmi::cpu::Cpu;
use crate::arm7tdmi::decoder::TransferLength;
use crate::memory::mmio::Mmio;

#[derive(Clone)]
pub struct MmioProxy(pub(super) *mut Mmio);

unsafe impl Send for MmioProxy {}
unsafe impl Sync for MmioProxy {}

impl MmioProxy {
    pub fn read_u8(&mut self, address: i64) -> u8 {
        unsafe { (*self.0).read(address as u32) }
    }

    pub fn read_u16(&mut self, address: i64) -> u16 {
        unsafe { (*self.0).read_u16(address as u32) }
    }

    pub fn read_u32(&mut self, address: i64) -> u32 {
        unsafe { (*self.0).read_u32(address as u32) }
    }

    pub fn write_u8(&mut self, address: i64, value: i64) {
        unsafe { (*self.0).write(address as u32, value as u8, TransferLength::Byte) }
    }

    pub fn write_u16(&mut self, address: i64, value: i64) {
        unsafe { (*self.0).write_u16(address as u32, value as u16) }
    }

    pub fn write_u32(&mut self, address: i64, value: i64) {
        unsafe { (*self.0).write_u32(address as u32, value as u32) }
    }
}

#[derive(Clone)]
pub struct CpuProxy(pub(super) *mut Cpu);

unsafe impl Send for CpuProxy {}
unsafe impl Sync for CpuProxy {}

impl CpuProxy {
    fn register_name_to_index(register: &str) -> usize {
        match register {
            "r0" => 0,
            "r1" => 1,
            "r2" => 2,
            "r3" => 3,
            "r4" => 4,
            "r5" => 5,
            "r6" => 6,
            "r7" => 7,
            "r8" => 8,
            "r9" => 9,
            "r10" => 10,
            "r11" => 11,
            "r12" => 12,
            "sp" | "r13" => 13,
            "lr" | "r14" => 14,
            "pc" | "r15" => 15,
            _ => panic!("Invalid register name: {}", register),
        }
    }

    pub fn read_register(&mut self, reg: &str) -> u32 {
        let reg = Self::register_name_to_index(reg);
        unsafe { (*self.0).registers.r[reg] }
    }

    pub fn write_register(&mut self, reg: &str, value: u32) {
        let reg = Self::register_name_to_index(reg);
        unsafe { (*self.0).registers.r[reg] = value }
    }

    pub fn read_cpsr(&mut self) -> u32 {
        unsafe { (*self.0).read_from_current_spsr().bits() }
    }
}
