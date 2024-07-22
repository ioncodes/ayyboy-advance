use std::fmt::{Debug, Display};

use crate::{
    arm7tdmi::{decoder::Opcode, handlers::Handlers},
    memory::mmio::Mmio,
};

use super::{
    decoder::{Instruction, Register},
    registers::{Cpsr, Registers},
};

pub struct Cpu {
    pub registers: Registers,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            registers: Registers::default(),
        }
    }

    pub fn tick(&mut self, mmio: &mut Mmio) {
        let pc = self.get_pc();
        let opcode = mmio.read_u32(pc);

        if self.is_thumb() {
            self.registers.r[15] += 2;
        } else {
            self.registers.r[15] += 4;
        }

        let instruction = Instruction::decode(opcode, self.is_thumb());
        println!(
            "{:08x} @ {:08x} | {:032b}: {:<20}\n{}\n",
            pc, opcode, opcode, instruction, self
        );

        match instruction.opcode {
            Opcode::B | Opcode::Bl | Opcode::Bx => Handlers::branch(&instruction, self, mmio),
            Opcode::Push | Opcode::Pop => Handlers::push_pop(&instruction, self, mmio),
            Opcode::Cmp | Opcode::Tst | Opcode::Teq | Opcode::Cmn => {
                Handlers::test(&instruction, self, mmio)
            }
            Opcode::Mov | Opcode::Mvn => Handlers::move_data(&instruction, self, mmio),
            _ => todo!(),
        }
    }

    pub fn read_register(&self, register: &Register) -> u32 {
        match register {
            Register::R0 => self.registers.r[0],
            Register::R1 => self.registers.r[1],
            Register::R2 => self.registers.r[2],
            Register::R3 => self.registers.r[3],
            Register::R4 => self.registers.r[4],
            Register::R5 => self.registers.r[5],
            Register::R6 => self.registers.r[6],
            Register::R7 => self.registers.r[7],
            Register::R8 => self.registers.r[8],
            Register::R9 => self.registers.r[9],
            Register::R10 => self.registers.r[10],
            Register::R11 => self.registers.r[11],
            Register::R12 => self.registers.r[12],
            Register::R13 => self.registers.r[13],
            Register::R14 => self.registers.r[14],
            Register::R15 => self.registers.r[15],
            _ => todo!(),
        }
    }

    pub fn write_register(&mut self, register: &Register, value: u32) {
        match register {
            Register::R0 => self.registers.r[0] = value,
            Register::R1 => self.registers.r[1] = value,
            Register::R2 => self.registers.r[2] = value,
            Register::R3 => self.registers.r[3] = value,
            Register::R4 => self.registers.r[4] = value,
            Register::R5 => self.registers.r[5] = value,
            Register::R6 => self.registers.r[6] = value,
            Register::R7 => self.registers.r[7] = value,
            Register::R8 => self.registers.r[8] = value,
            Register::R9 => self.registers.r[9] = value,
            Register::R10 => self.registers.r[10] = value,
            Register::R11 => self.registers.r[11] = value,
            Register::R12 => self.registers.r[12] = value,
            Register::R13 => self.registers.r[13] = value,
            Register::R14 => self.registers.r[14] = value,
            Register::R15 => self.registers.r[15] = value,
            _ => todo!(),
        }
    }

    pub fn update_flag(&mut self, flag: Cpsr, value: bool) {
        self.registers.cpsr.set(flag, value);
    }

    pub fn push_stack(&mut self, mmio: &mut Mmio, value: u32) {
        let sp = self.get_sp();
        let addr = sp.wrapping_sub(4);
        mmio.write_u32(addr, value);
        self.registers.r[13] = addr;
    }

    pub fn pop_stack(&mut self, mmio: &mut Mmio) -> u32 {
        let sp = self.get_sp();
        let value = mmio.read_u32(sp);
        self.registers.r[13] = sp.wrapping_add(4);
        value
    }

    // program counter
    pub fn get_pc(&self) -> u32 {
        self.registers.r[15]
    }

    // link register
    pub fn get_lr(&self) -> u32 {
        self.registers.r[14]
    }

    // stack pointer
    pub fn get_sp(&self) -> u32 {
        self.registers.r[13]
    }

    pub fn is_thumb(&self) -> bool {
        self.registers.cpsr.contains(Cpsr::T)
    }
}

impl Display for Cpu {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            " r0: {:08x}  r1: {:08x}  r2: {:08x}  r3: {:08x}\n",
            self.registers.r[0], self.registers.r[1], self.registers.r[2], self.registers.r[3]
        )?;
        write!(
            f,
            " r4: {:08x}  r5: {:08x}  r6: {:08x}  r7: {:08x}\n",
            self.registers.r[4], self.registers.r[5], self.registers.r[6], self.registers.r[7]
        )?;
        write!(
            f,
            " r8: {:08x}  r9: {:08x} r10: {:08x} r11: {:08x}\n",
            self.registers.r[8], self.registers.r[9], self.registers.r[10], self.registers.r[11]
        )?;
        write!(
            f,
            "r12: {:08x} r13: {:08x} r14: {:08x} r15: {:08x}\n",
            self.registers.r[12], self.registers.r[13], self.registers.r[14], self.registers.r[15]
        )?;
        write!(
            f,
            "spsr[0]: {}\nspsr[1]: {}\nspsr[2]: {}\nspsr[3]: {}\nspsr[4]: {}\n",
            self.registers.spsr[0],
            self.registers.spsr[1],
            self.registers.spsr[2],
            self.registers.spsr[3],
            self.registers.spsr[4]
        )?;
        write!(f, "cpsr: {}", self.registers.cpsr)
    }
}
