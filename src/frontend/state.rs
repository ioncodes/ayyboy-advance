use crate::arm7tdmi::{self};
use crate::memory;

#[derive(Default, Copy, Clone)]
pub struct Cpu {
    pub registers: [u32; 16],
    pub cpsr: arm7tdmi::registers::Psr,
}

#[derive(Clone)]
pub struct DbgState {
    pub cpu: Cpu,
    pub memory: Box<[u8; 0x0FFFFFFF + 1]>,
}

impl Default for DbgState {
    fn default() -> DbgState {
        let memory = Box::<[u8; 0x0FFFFFFF + 1]>::new_zeroed();

        DbgState {
            cpu: Cpu::default(),
            memory: unsafe { memory.assume_init() },
        }
    }
}

impl DbgState {
    pub fn from(cpu: &arm7tdmi::cpu::Cpu, mmio: &memory::mmio::Mmio) -> DbgState {
        let cpu = Cpu {
            registers: cpu.registers.r,
            cpsr: cpu.registers.cpsr,
        };
        let mut memory = unsafe {
            let memory = Box::<[u8; 0x0FFFFFFF + 1]>::new_zeroed();
            memory.assume_init()
        };
        memory[..=0x04FFFFFF].copy_from_slice(&mmio.internal_memory[..]);
        memory[0x05000000..=0x07FFFFFF].copy_from_slice(&mmio.ppu.vram[..]);
        memory[0x08000000..=0x0FFFFFFF].copy_from_slice(&mmio.external_memory[..]);

        DbgState { cpu, memory }
    }
}
