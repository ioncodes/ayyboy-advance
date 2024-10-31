use super::decoder::{Instruction, Register};
use super::mode::ProcessorMode;
use super::pipeline::{Pipeline, State};
use super::registers::{Psr, Registers};
use super::symbolizer::Symbolizer;
use crate::arm7tdmi::decoder::Opcode;
use crate::arm7tdmi::handlers::Handlers;
use crate::memory::mmio::Mmio;
use spdlog::prelude::*;
use std::fmt::Display;

pub struct Cpu {
    pub registers: Registers,
    pub pipeline: Pipeline,
    symbolizer: Symbolizer,
}

impl Cpu {
    pub fn new(buffer: &[u8]) -> Cpu {
        Cpu {
            registers: Registers::default(),
            pipeline: Pipeline::new(),
            symbolizer: Symbolizer::new(buffer),
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

            self.symbolizer.find(state.pc).map(|symbol| {
                debug!("Found matching symbols @ PC: {}", symbol.join(", "));
            });

            #[cfg(feature = "mesen2-trace-dump")]
            {
                println!("{:08X}  {:<45}  R0:{:08X} R1:{:08X} R2:{:08X} R3:{:08X} R4:{:08X} R5:{:08X} R6:{:08X} R7:{:08X} R8:{:08X} R9:{:08X} R10:{:08X} R11:{:08X} R12:{:08X} R13:{:08X} R14:{:08X} R15:{:08x} CPSR:{} Mode:{}",
                    state.pc,
                    format!("{}", instruction),
                    self.read_register(&Register::R0),
                    self.read_register(&Register::R1),
                    self.read_register(&Register::R2),
                    self.read_register(&Register::R3),
                    self.read_register(&Register::R4),
                    self.read_register(&Register::R5),
                    self.read_register(&Register::R6),
                    self.read_register(&Register::R7),
                    self.read_register(&Register::R8),
                    self.read_register(&Register::R9),
                    self.read_register(&Register::R10),
                    self.read_register(&Register::R11),
                    self.read_register(&Register::R12),
                    self.read_register(&Register::R13),
                    self.read_register(&Register::R14),
                    self.read_register(&Register::R15),
                    format!("{}{}{}{}{}{}{}", 
                        if self.registers.cpsr.contains(Psr::N) { "N" } else { "n" },
                        if self.registers.cpsr.contains(Psr::Z) { "Z" } else { "z" },
                        if self.registers.cpsr.contains(Psr::C) { "C" } else { "c" },
                        if self.registers.cpsr.contains(Psr::V) { "V" } else { "v" },
                        if self.registers.cpsr.contains(Psr::T) { "T" } else { "t" },
                        if self.registers.cpsr.contains(Psr::F) { "F" } else { "f" },
                        if self.registers.cpsr.contains(Psr::I) { "I" } else { "i" }
                    ),
                    match self.get_processor_mode() {
                        ProcessorMode::User => "USR",
                        ProcessorMode::Fiq => "FIQ",
                        ProcessorMode::Irq => "IRQ",
                        ProcessorMode::Supervisor => "SVC",
                        ProcessorMode::Abort => "ABT",
                        ProcessorMode::Undefined => "UND",
                        ProcessorMode::System => "SYS",
                    }
                );
            }

            match instruction.opcode {
                Opcode::B | Opcode::Bl | Opcode::Bx => Handlers::branch(&instruction, self, mmio),
                Opcode::Push | Opcode::Pop => Handlers::push_pop(&instruction, self, mmio),
                Opcode::Cmp | Opcode::Tst | Opcode::Teq | Opcode::Cmn => Handlers::test(&instruction, self, mmio),
                Opcode::Mov | Opcode::Mvn => Handlers::move_data(&instruction, self, mmio),
                Opcode::Ldm | Opcode::Stm | Opcode::Ldr | Opcode::Str | Opcode::Swp => {
                    Handlers::load_store(&instruction, self, mmio)
                }
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
                | Opcode::Umull
                | Opcode::Umlal
                | Opcode::Smull
                | Opcode::Smlal => Handlers::alu(&instruction, self, mmio),
                Opcode::Swi => Handlers::software_interrupt(&instruction, self, mmio),
                _ => todo!(),
            }

            trace!("\n{}", self);

            return Some((instruction, state));
        }

        None
    }

    pub fn read_register(&self, register: &Register) -> u32 {
        self.read_register_for_mode(register, self.get_processor_mode())
    }

    pub fn read_register_for_mode(&self, register: &Register, mode: ProcessorMode) -> u32 {
        match register {
            Register::R0 => self.registers.r[0],
            Register::R1 => self.registers.r[1],
            Register::R2 => self.registers.r[2],
            Register::R3 => self.registers.r[3],
            Register::R4 => self.registers.r[4],
            Register::R5 => self.registers.r[5],
            Register::R6 => self.registers.r[6],
            Register::R7 => self.registers.r[7],
            Register::R8 => match mode {
                ProcessorMode::Fiq => self.registers.bank[&ProcessorMode::Fiq][0],
                _ => self.registers.r[8],
            },
            Register::R9 => match mode {
                ProcessorMode::Fiq => self.registers.bank[&ProcessorMode::Fiq][1],
                _ => self.registers.r[9],
            },
            Register::R10 => match mode {
                ProcessorMode::Fiq => self.registers.bank[&ProcessorMode::Fiq][2],
                _ => self.registers.r[10],
            },
            Register::R11 => match mode {
                ProcessorMode::Fiq => self.registers.bank[&ProcessorMode::Fiq][3],
                _ => self.registers.r[11],
            },
            Register::R12 => match mode {
                ProcessorMode::Fiq => self.registers.bank[&ProcessorMode::Fiq][4],
                _ => self.registers.r[12],
            },
            Register::R13 => match mode {
                ProcessorMode::Fiq => self.registers.bank[&ProcessorMode::Fiq][5],
                ProcessorMode::Supervisor => self.registers.bank[&ProcessorMode::Supervisor][0],
                ProcessorMode::Abort => self.registers.bank[&ProcessorMode::Abort][0],
                ProcessorMode::Irq => self.registers.bank[&ProcessorMode::Irq][0],
                ProcessorMode::Undefined => self.registers.bank[&ProcessorMode::Undefined][0],
                _ => self.registers.r[13],
            },
            Register::R14 => match mode {
                ProcessorMode::Fiq => self.registers.bank[&ProcessorMode::Fiq][6],
                ProcessorMode::Supervisor => self.registers.bank[&ProcessorMode::Supervisor][1],
                ProcessorMode::Abort => self.registers.bank[&ProcessorMode::Abort][1],
                ProcessorMode::Irq => self.registers.bank[&ProcessorMode::Irq][1],
                ProcessorMode::Undefined => self.registers.bank[&ProcessorMode::Undefined][1],
                _ => self.registers.r[14],
            },
            Register::R15 => {
                // WhenGryphonsFly — Today at 1:51 PM
                // In thumb mode, PC-relative loads treat bit 1 of PC as always 0
                if self.is_thumb() {
                    self.registers.r[15] & !0b10
                } else {
                    self.registers.r[15]
                }
            }
            Register::Cpsr => self.registers.cpsr.bits(),
            Register::Spsr => self.read_from_current_spsr().bits(),
            _ => todo!(),
        }
    }

    pub fn write_register(&mut self, register: &Register, value: u32) {
        self.write_register_for_mode(register, value, self.get_processor_mode());
    }

    pub fn write_register_for_mode(&mut self, register: &Register, value: u32, mode: ProcessorMode) {
        match register {
            Register::R0 => self.registers.r[0] = value,
            Register::R1 => self.registers.r[1] = value,
            Register::R2 => self.registers.r[2] = value,
            Register::R3 => self.registers.r[3] = value,
            Register::R4 => self.registers.r[4] = value,
            Register::R5 => self.registers.r[5] = value,
            Register::R6 => self.registers.r[6] = value,
            Register::R7 => self.registers.r[7] = value,
            Register::R8 => match mode {
                ProcessorMode::Fiq => self.registers.bank.get_mut(&ProcessorMode::Fiq).unwrap()[0] = value,
                _ => self.registers.r[8] = value,
            },
            Register::R9 => match mode {
                ProcessorMode::Fiq => self.registers.bank.get_mut(&ProcessorMode::Fiq).unwrap()[1] = value,
                _ => self.registers.r[9] = value,
            },
            Register::R10 => match mode {
                ProcessorMode::Fiq => self.registers.bank.get_mut(&ProcessorMode::Fiq).unwrap()[2] = value,
                _ => self.registers.r[10] = value,
            },
            Register::R11 => match mode {
                ProcessorMode::Fiq => self.registers.bank.get_mut(&ProcessorMode::Fiq).unwrap()[3] = value,
                _ => self.registers.r[11] = value,
            },
            Register::R12 => match mode {
                ProcessorMode::Fiq => self.registers.bank.get_mut(&ProcessorMode::Fiq).unwrap()[4] = value,
                _ => self.registers.r[12] = value,
            },
            Register::R13 => match mode {
                ProcessorMode::Fiq => self.registers.bank.get_mut(&ProcessorMode::Fiq).unwrap()[5] = value,
                ProcessorMode::Supervisor => {
                    self.registers.bank.get_mut(&ProcessorMode::Supervisor).unwrap()[0] = value
                }
                ProcessorMode::Abort => self.registers.bank.get_mut(&ProcessorMode::Abort).unwrap()[0] = value,
                ProcessorMode::Irq => self.registers.bank.get_mut(&ProcessorMode::Irq).unwrap()[0] = value,
                ProcessorMode::Undefined => self.registers.bank.get_mut(&ProcessorMode::Undefined).unwrap()[0] = value,
                _ => self.registers.r[13] = value,
            },
            Register::R14 => match mode {
                ProcessorMode::Fiq => self.registers.bank.get_mut(&ProcessorMode::Fiq).unwrap()[6] = value,
                ProcessorMode::Supervisor => {
                    self.registers.bank.get_mut(&ProcessorMode::Supervisor).unwrap()[1] = value
                }
                ProcessorMode::Abort => self.registers.bank.get_mut(&ProcessorMode::Abort).unwrap()[1] = value,
                ProcessorMode::Irq => self.registers.bank.get_mut(&ProcessorMode::Irq).unwrap()[1] = value,
                ProcessorMode::Undefined => self.registers.bank.get_mut(&ProcessorMode::Undefined).unwrap()[1] = value,
                _ => self.registers.r[14] = value,
            },
            Register::R15 => {
                // since PC is a GP register, it can be freely written to
                // we need to flush the pipeline if that's the case
                self.registers.r[15] = value;
                self.pipeline.flush();
            }
            Register::Cpsr => {
                self.registers.cpsr =
                    Psr::from_bits_truncate((self.registers.cpsr.bits() & Psr::M.bits()) | (value & !Psr::M.bits()));
                let new_mode = ProcessorMode::from(value & Psr::M.bits());
                self.set_processor_mode(new_mode);
            }
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
                let new_mode = ProcessorMode::from((cpsr & Psr::M).bits());
                self.set_processor_mode(new_mode);
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
            Register::Spsr => self.write_to_current_spsr(Psr::from_bits_truncate(value)),
            Register::SpsrFlag => {
                let mut current = self.read_from_current_spsr();
                let spsr = Psr::from_bits_truncate(value);
                current.set(Psr::N, spsr.contains(Psr::N));
                current.set(Psr::Z, spsr.contains(Psr::Z));
                current.set(Psr::C, spsr.contains(Psr::C));
                current.set(Psr::V, spsr.contains(Psr::V));
                self.write_to_current_spsr(current);
            }
            Register::SpsrControl => {
                let mut current = self.read_from_current_spsr();
                let spsr = Psr::from_bits_truncate(value);
                current.set(Psr::I, spsr.contains(Psr::I));
                current.set(Psr::F, spsr.contains(Psr::F));
                current.set(Psr::T, spsr.contains(Psr::T));
                current = Psr::from_bits_truncate((current.bits() & !Psr::M.bits()) | (spsr.bits() & Psr::M.bits()));
                self.write_to_current_spsr(current);
            }
            Register::SpsrFlagControl => {
                let mut current = self.read_from_current_spsr();
                let spsr = Psr::from_bits_truncate(value);

                // update flags
                current.set(Psr::N, spsr.contains(Psr::N));
                current.set(Psr::Z, spsr.contains(Psr::Z));
                current.set(Psr::C, spsr.contains(Psr::C));
                current.set(Psr::V, spsr.contains(Psr::V));

                // update control bits
                current.set(Psr::I, spsr.contains(Psr::I));
                current.set(Psr::F, spsr.contains(Psr::F));
                current.set(Psr::T, spsr.contains(Psr::T));

                // mode switch
                current = Psr::from_bits_truncate((current.bits() & !Psr::M.bits()) | (spsr.bits() & Psr::M.bits()));
                self.write_to_current_spsr(current);
            }
            Register::PsrNone => {
                // basically a nop
            }
        }
    }

    pub fn update_flag(&mut self, flag: Psr, value: bool) {
        self.registers.cpsr.set(flag, value);
    }

    pub fn push_stack(&mut self, mmio: &mut Mmio, value: u32) {
        let sp = self.get_sp();
        let addr = sp.wrapping_sub(4);
        mmio.write_u32(addr, value);
        self.write_register(&Register::R13, addr);
    }

    pub fn pop_stack(&mut self, mmio: &mut Mmio) -> u32 {
        let sp = self.get_sp();
        let value = mmio.read_u32(sp);
        self.write_register(&Register::R13, sp.wrapping_add(4));
        value
    }

    // program counter
    pub fn get_pc(&self) -> u32 {
        // this needs direct access as read_register
        // may ignore bit 1 in thumb mode
        self.registers.r[15]
    }

    // stack pointer
    pub fn get_sp(&self) -> u32 {
        self.read_register(&Register::R13)
    }

    pub fn get_processor_mode(&self) -> ProcessorMode {
        let mode = self.registers.cpsr.bits() & Psr::M.bits();
        ProcessorMode::from(mode)
    }

    pub fn set_processor_mode(&mut self, mode: ProcessorMode) {
        let current_mode = self.get_processor_mode();
        self.registers.cpsr =
            Psr::from_bits_truncate((self.registers.cpsr.bits() & !Psr::M.bits()) | ((mode as u32) & Psr::M.bits()));
        debug!("Switched from {} to {}", current_mode, mode);
    }

    pub fn write_to_current_spsr(&mut self, value: Psr) {
        let mode = self.get_processor_mode();
        self.write_to_spsr(mode, value);
    }

    pub fn write_to_spsr(&mut self, mode: ProcessorMode, value: Psr) {
        if mode == ProcessorMode::User || mode == ProcessorMode::System {
            error!("Attempted to write to User/System SPSR");
            return;
        }

        match mode {
            ProcessorMode::Fiq => self.registers.spsr[0] = value,
            ProcessorMode::Supervisor => self.registers.spsr[1] = value,
            ProcessorMode::Abort => self.registers.spsr[2] = value,
            ProcessorMode::Irq => self.registers.spsr[3] = value,
            ProcessorMode::Undefined => self.registers.spsr[4] = value,
            _ => todo!(),
        }
    }

    pub fn read_from_current_spsr(&self) -> Psr {
        let mode = self.get_processor_mode();
        self.read_from_spsr(mode)
    }

    pub fn read_from_spsr(&self, mode: ProcessorMode) -> Psr {
        match mode {
            ProcessorMode::User | ProcessorMode::System => {
                error!("Attempted to read from User/System SPSR");
                self.registers.cpsr
            }
            _ => match mode {
                ProcessorMode::Fiq => self.registers.spsr[0],
                ProcessorMode::Supervisor => self.registers.spsr[1],
                ProcessorMode::Abort => self.registers.spsr[2],
                ProcessorMode::Irq => self.registers.spsr[3],
                ProcessorMode::Undefined => self.registers.spsr[4],
                _ => todo!(),
            },
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
            self.read_register(&Register::R0),
            self.read_register(&Register::R1),
            self.read_register(&Register::R2),
            self.read_register(&Register::R3)
        )?;
        write!(
            f,
            " r4: {:08x}  r5: {:08x}  r6: {:08x}  r7: {:08x}\n",
            self.read_register(&Register::R4),
            self.read_register(&Register::R5),
            self.read_register(&Register::R6),
            self.read_register(&Register::R7)
        )?;
        write!(
            f,
            " r8: {:08x}  r9: {:08x} r10: {:08x} r11: {:08x}\n",
            self.read_register(&Register::R8),
            self.read_register(&Register::R9),
            self.read_register(&Register::R10),
            self.read_register(&Register::R11)
        )?;
        write!(
            f,
            "r12: {:08x} r13: {:08x} r14: {:08x} r15: {:08x}\n",
            self.read_register(&Register::R12),
            self.read_register(&Register::R13),
            self.read_register(&Register::R14),
            self.read_register(&Register::R15)
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
            "spsr[0]: {}{{{},{}}}\nspsr[1]: {}{{{},{}}}\nspsr[2]: {}{{{},{}}}\nspsr[3]: {}{{{},{}}}\nspsr[4]: {}{{{},{}}}",
            self.registers.spsr[0],
            if self.registers.spsr[0].contains(Psr::T) { "Thumb" } else { "Arm" },
            self.registers.spsr[0].mode(),
            self.registers.spsr[1],
            if self.registers.spsr[1].contains(Psr::T) { "Thumb" } else { "Arm" },
            self.registers.spsr[1].mode(),
            self.registers.spsr[2],
            if self.registers.spsr[2].contains(Psr::T) { "Thumb" } else { "Arm" },
            self.registers.spsr[2].mode(),
            self.registers.spsr[3],
            if self.registers.spsr[3].contains(Psr::T) { "Thumb" } else { "Arm" },
            self.registers.spsr[3].mode(),
            self.registers.spsr[4],
            if self.registers.spsr[4].contains(Psr::T) { "Thumb" } else { "Arm" },
            self.registers.spsr[4].mode()
        )
    }
}
