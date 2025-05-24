use crate::arm7tdmi::cpu::Cpu;
use std::marker::PhantomData;

pub struct Proxy {
    cpu_ptr: *mut Cpu,
    _marker: PhantomData<Cpu>,
}

unsafe impl Send for Proxy {}
unsafe impl Sync for Proxy {}

impl Clone for Proxy {
    fn clone(&self) -> Self {
        Self {
            cpu_ptr: self.cpu_ptr,
            _marker: PhantomData,
        }
    }
}

impl Proxy {
    pub fn new(cpu: &mut Cpu) -> Self {
        Self {
            cpu_ptr: cpu as *mut Cpu,
            _marker: PhantomData,
        }
    }

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

    pub fn read_register(&self, reg: &str) -> u32 {
        let reg_index = Self::register_name_to_index(reg);
        unsafe { (*self.cpu_ptr).registers.r[reg_index] }
    }

    pub fn write_register(&mut self, reg: &str, value: u32) {
        let reg_index = Self::register_name_to_index(reg);
        unsafe {
            (*self.cpu_ptr).registers.r[reg_index] = value;
        }
    }

    pub fn read_cpsr(&self) -> u32 {
        unsafe { (*self.cpu_ptr).read_from_current_spsr().bits() }
    }

    pub fn read_u8(&self, address: i64) -> u8 {
        unsafe { (*self.cpu_ptr).mmio.read(address as u32) }
    }

    pub fn read_u16(&self, address: i64) -> u16 {
        unsafe { (*self.cpu_ptr).mmio.read_u16(address as u32) }
    }

    pub fn read_u32(&self, address: i64) -> u32 {
        unsafe { (*self.cpu_ptr).mmio.read_u32(address as u32) }
    }

    pub fn write_u8(&mut self, address: i64, value: i64) {
        unsafe { (*self.cpu_ptr).mmio.write(address as u32, value as u8) }
    }

    pub fn write_u16(&mut self, address: i64, value: i64) {
        unsafe { (*self.cpu_ptr).mmio.write_u16(address as u32, value as u16) }
    }

    pub fn write_u32(&mut self, address: i64, value: i64) {
        unsafe { (*self.cpu_ptr).mmio.write_u32(address as u32, value as u32) }
    }
}
