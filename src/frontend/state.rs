use crate::arm7tdmi;

#[derive(Default, Copy, Clone)]
pub struct Cpu {
    pub registers: [u32; 16],
}

#[derive(Default, Copy, Clone)]
pub struct DbgState {
    pub cpu: Cpu,
}

impl From<&arm7tdmi::cpu::Cpu> for DbgState {
    fn from(cpu: &arm7tdmi::cpu::Cpu) -> Self {
        let cpu = Cpu {
            registers: cpu.registers.r,
        };

        Self { cpu }
    }
}
