use std::fmt::Display;

use bitflags::Flags;

use crate::{
    arm7tdmi::{decoder::Opcode, handlers::Handlers},
    memory::mmio::Mmio,
};

use super::{
    decoder::{Instruction, Register},
    registers::{Psr, Registers},
};

#[derive(Debug)]
pub enum ProcessorMode {
    User,
    Fiq,
    Irq,
    Supervisor,
    Abort,
    System,
}

impl Into<u32> for ProcessorMode {
    fn into(self) -> u32 {
        match self {
            ProcessorMode::User => 0b10000,
            ProcessorMode::Fiq => 0b10001,
            ProcessorMode::Irq => 0b10010,
            ProcessorMode::Supervisor => 0b10011,
            ProcessorMode::Abort => 0b10111,
            ProcessorMode::System => 0b11111,
        }
    }
}

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
        let real_pc = self.get_real_pc();

        let opcode = if self.is_thumb() {
            self.registers.r[15] += 2;
            mmio.read_u16(real_pc) as u32
        } else {
            self.registers.r[15] += 4;
            mmio.read_u32(real_pc)
        };

        let instruction = Instruction::decode(opcode, self.is_thumb());
        if self.is_thumb() {
            println!(
                "{:08x} @ {:04x} | {:016b}: {}",
                real_pc, opcode, opcode, instruction
            );
        } else {
            println!(
                "{:08x} @ {:08x} | {:032b}: {}",
                real_pc, opcode, opcode, instruction
            );
        }

        match instruction.opcode {
            Opcode::B | Opcode::Bl | Opcode::Bx => Handlers::branch(&instruction, self, mmio),
            Opcode::Push | Opcode::Pop => Handlers::push_pop(&instruction, self, mmio),
            Opcode::Cmp | Opcode::Tst | Opcode::Teq | Opcode::Cmn => {
                Handlers::test(&instruction, self, mmio)
            }
            Opcode::Mov | Opcode::Mvn => Handlers::move_data(&instruction, self, mmio),
            Opcode::Ldm | Opcode::Stm | Opcode::Ldr | Opcode::Str => {
                Handlers::load_store(&instruction, self, mmio)
            }
            Opcode::Mrs | Opcode::Msr => Handlers::psr_transfer(&instruction, self, mmio),
            Opcode::Add | Opcode::Sub | Opcode::And | Opcode::Orr | Opcode::Eor | Opcode::Rsb => {
                Handlers::alu(&instruction, self, mmio)
            }
            _ => todo!(),
        }

        println!("{}\n", self);
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
            Register::Cpsr => self.registers.cpsr.bits(),
            Register::Spsr => self.read_from_current_spsr(),
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
            Register::Cpsr => self.registers.cpsr = Psr::from_bits_truncate(value),
            Register::Spsr => self.write_to_current_spsr(value),
            _ => todo!(),
        }
    }

    pub fn write_register_u8(&mut self, register: &Register, value: u8) {
        let original_value = self.read_register(register);
        self.write_register(register, (original_value & 0xffffff00) | value as u32);
    }

    pub fn update_flag(&mut self, flag: Psr, value: bool) {
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

    // program counter, pipeline effect. only account for 1 instruction
    // 2nd instruction is accounted for in the tick function
    pub fn get_pc(&self) -> u32 {
        self.registers.r[15] + 4
    }

    // program counter, real value
    pub fn get_real_pc(&self) -> u32 {
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

    pub fn get_processor_mode(&self) -> ProcessorMode {
        let mode = self.registers.cpsr.bits() & Psr::M.bits();
        match mode {
            0b10000 => ProcessorMode::User,
            0b10001 => ProcessorMode::Fiq,
            0b10010 => ProcessorMode::Irq,
            0b10011 => ProcessorMode::Supervisor,
            0b10111 => ProcessorMode::Abort,
            0b11111 => ProcessorMode::System,
            _ => unreachable!(),
        }
    }

    pub fn set_processor_mode(&mut self, mode: ProcessorMode) {
        let mode: u32 = mode.into();
        self.registers.cpsr =
            Psr::from_bits_truncate((self.registers.cpsr.bits() & !Psr::M.bits()) | mode);
    }

    pub fn write_to_current_spsr(&mut self, value: u32) {
        let mode = self.get_processor_mode();
        match mode {
            ProcessorMode::User | ProcessorMode::System => return,
            _ => (),
        }

        let mode: u32 = mode.into();
        let spsr = &mut self.registers.spsr[mode as usize - 0b10001];
        *spsr = Psr::from_bits_truncate(value);
    }

    pub fn read_from_current_spsr(&self) -> u32 {
        let mode = self.get_processor_mode();
        match mode {
            ProcessorMode::User | ProcessorMode::System => 0,
            _ => self.registers.spsr[mode as usize - 0b10001].bits(),
        }
    }

    pub fn is_thumb(&self) -> bool {
        self.registers.cpsr.contains(Psr::T)
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
            "cpsr: {} {{{:?},{}}}\n",
            self.registers.cpsr,
            self.get_processor_mode(),
            if self.is_thumb() { "Thumb" } else { "Arm" }
        )?;
        write!(
            f,
            "spsr[0]: {}\nspsr[1]: {}\nspsr[2]: {}\nspsr[3]: {}\nspsr[4]: {}\n",
            self.registers.spsr[0],
            self.registers.spsr[1],
            self.registers.spsr[2],
            self.registers.spsr[3],
            self.registers.spsr[4]
        )
    }
}
