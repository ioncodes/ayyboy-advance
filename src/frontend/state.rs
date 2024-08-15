use crate::arm7tdmi::registers::Psr;
use crate::arm7tdmi::{self};

#[derive(Default, Copy, Clone)]
pub struct Cpu {
    pub registers: [u32; 16],
    pub cpsr: Psr,
}

#[derive(Default, Copy, Clone)]
pub struct DbgState {
    pub cpu: Cpu,
}

impl From<&arm7tdmi::cpu::Cpu> for DbgState {
    fn from(cpu: &arm7tdmi::cpu::Cpu) -> DbgState {
        let cpu = Cpu {
            registers: cpu.registers.r,
            cpsr: cpu.registers.cpsr,
        };

        DbgState { cpu }
    }
}
