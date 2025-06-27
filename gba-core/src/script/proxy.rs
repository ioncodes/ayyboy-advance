use crate::arm7tdmi::cpu::Cpu;
use crate::arm7tdmi::decoder::Register;
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

    fn parse_register(register: &str) -> Register {
        match register {
            "r0" => Register::R0,
            "r1" => Register::R1,
            "r2" => Register::R2,
            "r3" => Register::R3,
            "r4" => Register::R4,
            "r5" => Register::R5,
            "r6" => Register::R6,
            "r7" => Register::R7,
            "r8" => Register::R8,
            "r9" => Register::R9,
            "r10" => Register::R10,
            "r11" => Register::R11,
            "r12" => Register::R12,
            "sp" | "r13" => Register::R13,
            "lr" | "r14" => Register::R14,
            "pc" | "r15" => Register::R15,
            _ => panic!("Invalid register name: {}", register),
        }
    }

    pub fn read_register(&self, reg: &str) -> u32 {
        let register = Self::parse_register(reg);
        unsafe { (*self.cpu_ptr).read_register(&register) }
    }

    pub fn write_register(&mut self, reg: &str, value: u32) {
        let register = Self::parse_register(reg);
        unsafe {
            (*self.cpu_ptr).write_register(&register, value);
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

    pub fn is_thumb(&self) -> bool {
        unsafe { (*self.cpu_ptr).is_thumb() }
    }
}
