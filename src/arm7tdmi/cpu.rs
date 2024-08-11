use std::fmt::Display;

use log::{debug, error, trace};

use crate::arm7tdmi::decoder::Opcode;
use crate::arm7tdmi::handlers::Handlers;
use crate::memory::mmio::Mmio;

use super::decoder::{Instruction, Register};
use super::pipeline::{Pipeline, State};
use super::registers::{Psr, Registers};

#[derive(Debug)]
pub enum ProcessorMode {
    User = 0b10000,
    Fiq = 0b10001,
    Irq = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
    System = 0b11111,
}

impl ProcessorMode {
    pub fn from(value: u32) -> ProcessorMode {
        match value {
            0b10000 => ProcessorMode::User,
            0b10001 => ProcessorMode::Fiq,
            0b10010 => ProcessorMode::Irq,
            0b10011 => ProcessorMode::Supervisor,
            0b10111 => ProcessorMode::Abort,
            0b11111 => ProcessorMode::System,
            _ => panic!("Invalid processor mode: {:08b}", value),
        }
    }
}

pub struct Cpu {
    pub registers: Registers,
    pub pipeline: Pipeline,
}

impl Cpu {
    pub fn new() -> Cpu {
        Cpu {
            registers: Registers::default(),
            pipeline: Pipeline::new(),
        }
    }

    pub fn tick(&mut self, mmio: &mut Mmio) -> Option<(Instruction, State)> {
        self.pipeline.advance(self.get_pc(), self.is_thumb(), mmio);
        trace!("Pipeline: {}", self.pipeline);

        if self.is_thumb() {
            self.registers.r[15] += 2;
        } else {
            self.registers.r[15] += 4;
        }

        if let Some((instruction, state)) = self.pipeline.pop() {
            trace!("Instruction: {:?}", instruction);
            if self.is_thumb() {
                trace!("Opcode: {:04x} | {:016b}", state.opcode, state.opcode);
                debug!("{:08x}: {}", state.pc, instruction);
            } else {
                trace!("Opcode: {:08x} | {:032b}", state.opcode, state.opcode);
                debug!("{:08x}: {}", state.pc, instruction);
            }

            match instruction.opcode {
                Opcode::B | Opcode::Bl | Opcode::Bx => Handlers::branch(&instruction, self, mmio),
                Opcode::Push | Opcode::Pop => Handlers::push_pop(&instruction, self, mmio),
                Opcode::Cmp | Opcode::Tst | Opcode::Teq | Opcode::Cmn => Handlers::test(&instruction, self, mmio),
                Opcode::Mov | Opcode::Mvn => Handlers::move_data(&instruction, self, mmio),
                Opcode::Ldm | Opcode::Stm | Opcode::Ldr | Opcode::Str => Handlers::load_store(&instruction, self, mmio),
                Opcode::Mrs | Opcode::Msr => Handlers::psr_transfer(&instruction, self, mmio),
                Opcode::Add
                | Opcode::Adc
                | Opcode::Sub
                | Opcode::Sbc
                | Opcode::Rsc
                | Opcode::And
                | Opcode::Orr
                | Opcode::Eor
                | Opcode::Rsb
                | Opcode::Bic
                | Opcode::Neg
                | Opcode::Asr
                | Opcode::Lsl
                | Opcode::Lsr
                | Opcode::Mul
                | Opcode::Mla
                | Opcode::Mull
                | Opcode::Mlal => Handlers::alu(&instruction, self, mmio),
                Opcode::Swi => Handlers::software_interrupt(&instruction, self, mmio),
                _ => todo!(),
            }

            trace!("\n{}", self);

            return Some((instruction, state));
        }

        None
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
            Register::R15 => {
                let pc = self.registers.r[15];
                if self.is_thumb() {
                    // WhenGryphonsFly — Today at 1:51 PM
                    // In thumb mode, PC-relative loads treat bit 1 of PC as always 0
                    // TODO: does this have negative side effects if handled here? if it does,
                    // we should handle it in the load/store handler
                    pc & !0b10
                } else {
                    pc
                }
            }
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
            Register::R15 => {
                // since PC is a GP register, it can be freely written to
                // we need to flush the pipeline if that's the case
                self.registers.r[15] = value;
                self.pipeline.flush();
            }
            Register::Cpsr => self.registers.cpsr = Psr::from_bits_truncate(value),
            Register::CpsrFlag => {
                let cpsr = Psr::from_bits_truncate(value);
                self.update_flag(Psr::N, cpsr.contains(Psr::N));
                self.update_flag(Psr::Z, cpsr.contains(Psr::Z));
                self.update_flag(Psr::C, cpsr.contains(Psr::C));
                self.update_flag(Psr::V, cpsr.contains(Psr::V));
            }
            Register::CpsrControl => {
                let cpsr = Psr::from_bits_truncate(value);
                self.update_flag(Psr::I, cpsr.contains(Psr::I));
                self.update_flag(Psr::F, cpsr.contains(Psr::F));
                self.update_flag(Psr::T, cpsr.contains(Psr::T));
                self.registers.cpsr = (self.registers.cpsr & !Psr::M) | (cpsr & Psr::M);
            }
            Register::CpsrFlagControl => {
                let cpsr = Psr::from_bits_truncate(value);

                // update flags
                self.update_flag(Psr::N, cpsr.contains(Psr::N));
                self.update_flag(Psr::Z, cpsr.contains(Psr::Z));
                self.update_flag(Psr::C, cpsr.contains(Psr::C));
                self.update_flag(Psr::V, cpsr.contains(Psr::V));

                // update control bits
                self.update_flag(Psr::I, cpsr.contains(Psr::I));
                self.update_flag(Psr::F, cpsr.contains(Psr::F));
                self.update_flag(Psr::T, cpsr.contains(Psr::T));

                // switch mode
                let new_mode = ProcessorMode::from((cpsr & Psr::M).bits());
                self.set_processor_mode(new_mode);
            }
            Register::Spsr => self.write_to_current_spsr(value),
            Register::SpsrFlag => {
                let mut current = self.read_from_current_spsr();
                let spsr = Psr::from_bits_truncate(value);
                current = (current & !Psr::N.bits()) | (spsr & Psr::N).bits();
                current = (current & !Psr::Z.bits()) | (spsr & Psr::Z).bits();
                current = (current & !Psr::C.bits()) | (spsr & Psr::C).bits();
                current = (current & !Psr::V.bits()) | (spsr & Psr::V).bits();
                self.write_to_current_spsr(current);
            }
            Register::SpsrControl => {
                let mut current = self.read_from_current_spsr();
                let spsr = Psr::from_bits_truncate(value);
                current = (current & !Psr::I.bits()) | (spsr & Psr::I).bits();
                current = (current & !Psr::F.bits()) | (spsr & Psr::F).bits();
                current = (current & !Psr::T.bits()) | (spsr & Psr::T).bits();
                current = (current & !Psr::M.bits()) | (spsr & Psr::M).bits();
                self.write_to_current_spsr(current);
            }
            Register::SpsrFlagControl => {
                let mut current = self.read_from_current_spsr();
                let spsr = Psr::from_bits_truncate(value);

                // update flags
                current = (current & !Psr::N.bits()) | (spsr & Psr::N).bits();
                current = (current & !Psr::Z.bits()) | (spsr & Psr::Z).bits();
                current = (current & !Psr::C.bits()) | (spsr & Psr::C).bits();
                current = (current & !Psr::V.bits()) | (spsr & Psr::V).bits();

                // update control bits
                current = (current & !Psr::I.bits()) | (spsr & Psr::I).bits();
                current = (current & !Psr::F.bits()) | (spsr & Psr::F).bits();
                current = (current & !Psr::T.bits()) | (spsr & Psr::T).bits();

                // mode switch
                current = (current & !Psr::M.bits()) | (spsr & Psr::M).bits();
                self.write_to_current_spsr(current);
            }
        }
    }

    pub fn write_register_u8(&mut self, register: &Register, value: u8) {
        let original_value = self.read_register(register);
        self.write_register(register, (original_value & 0xffffff00) | value as u32);
    }

    pub fn write_register_u16(&mut self, register: &Register, value: u16) {
        let original_value = self.read_register(register);
        self.write_register(register, (original_value & 0xffff0000) | value as u32);
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

    // program counter, pipeline effect
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

    pub fn get_processor_mode(&self) -> ProcessorMode {
        let mode = self.registers.cpsr.bits() & Psr::M.bits();
        ProcessorMode::from(mode)
    }

    pub fn set_processor_mode(&mut self, mode: ProcessorMode) {
        let mode = mode as u32;
        self.registers.cpsr = Psr::from_bits_truncate((self.registers.cpsr.bits() & !Psr::M.bits()) | mode);
    }

    pub fn write_to_current_spsr(&mut self, value: u32) {
        let mode = self.get_processor_mode();
        match mode {
            ProcessorMode::User | ProcessorMode::System => {
                error!("Attempted to write to SPSR in User/System mode");
                return;
            }
            _ => (),
        }

        let mode = mode as usize;
        let spsr = &mut self.registers.spsr[mode - 0b10001];
        *spsr = Psr::from_bits_truncate(value);
    }

    pub fn read_from_current_spsr(&self) -> u32 {
        let mode = self.get_processor_mode();
        match mode {
            ProcessorMode::User | ProcessorMode::System => 0,
            _ => {
                let mode = mode as usize;
                self.registers.spsr[mode - 0b10001].bits()
            }
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
            "spsr[0]: {}\nspsr[1]: {}\nspsr[2]: {}\nspsr[3]: {}\nspsr[4]: {}",
            self.registers.spsr[0],
            self.registers.spsr[1],
            self.registers.spsr[2],
            self.registers.spsr[3],
            self.registers.spsr[4]
        )
    }
}
