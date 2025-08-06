use super::decoder::{Instruction, Register};
use super::mode::ProcessorMode;
use super::pipeline::{Pipeline, State};
use super::registers::{Psr, Registers};
use super::symbolizer::Symbolizer;
use crate::arm7tdmi::decoder::Opcode;
use crate::arm7tdmi::error::CpuError;
use crate::arm7tdmi::handlers::Handlers;
use crate::memory::device::IoRegister;
use crate::memory::mmio::Mmio;
use std::fmt::Display;
use tracing::*;

pub struct Cpu {
    pub registers: Registers,
    pub pipeline: Pipeline,
    pub mmio: Mmio,
    symbolizer: Symbolizer,
}

impl Cpu {
    pub fn new(buffer: &[u8], mmio: Mmio) -> Cpu {
        Cpu {
            registers: Registers::default(),
            pipeline: Pipeline::new(),
            mmio,
            symbolizer: Symbolizer::new(buffer),
        }
    }

    pub fn tick(&mut self) -> Result<(Instruction, State), CpuError> {
        let IoRegister(ime_value) = self.mmio.io_ime;
        let IoRegister(halt_cnt) = self.mmio.io_halt_cnt;

        // TODO: do we need the IRQ check here?
        if self.get_pc() < 0x0000_4000 || self.get_processor_mode() == ProcessorMode::Irq {
            self.mmio.enable_bios_access();
        } else {
            self.mmio.disable_bios_access();
        }

        self.pipeline.advance(self.get_pc(), self.is_thumb(), &mut self.mmio);
        trace!(target: "pipeline", "Pipeline: {}", self.pipeline);

        // Check for any pending interrupts that are both requested (IF) and enabled (IE)
        let pending_interrupts = self.mmio.io_if.value().bits() & self.mmio.io_ie.value().bits();

        // we need to make sure the pipeline is full before we trigger an IRQ
        // the IRQ always returns using subs pc, lr, #4, so if the pipeline has been flushed recently
        // PC = current instruction, so on return we get current instruction - 4 which is behind the current instruction
        if ime_value != 0
            && pending_interrupts != 0
            && !self.registers.cpsr.contains(Psr::I)
            && self.pipeline.is_full()
        {
            trace!(target: "irq", "IRQ available, switching to IRQ mode");

            // copy CPSR to SPSR and switch to IRQ mode
            self.write_to_spsr(ProcessorMode::Irq, self.registers.cpsr);
            self.set_processor_mode(ProcessorMode::Irq);

            // write LR and jump to IRQ vector
            self.write_register(
                &Register::R14,
                if self.is_thumb() {
                    self.get_pc()
                } else {
                    self.get_pc() - 4
                },
            );
            self.write_register(&Register::R15, 0x18);

            // disable interrupts and switch to ARM
            self.registers.cpsr.set(Psr::I, true);
            self.registers.cpsr.set(Psr::T, false);

            //self.pipeline.flush(); VERIFYME: we don't have to flush, write register R15 will do it for us

            // allow cpu to continue
            self.mmio.io_halt_cnt.set(0xff);

            return Err(CpuError::InterruptTriggered);
        }

        // TODO: 0x80 is STOP MODE, it should be handled differently
        // We need to check this AFTER the IRQ check, or else we will never enter
        // another IRQ during halt
        if halt_cnt == 0 {
            trace!(target: "cpu", "CPU is halted");
            return Err(CpuError::CpuPaused);
        }

        if let Some((instruction, state)) = self.pipeline.pop() {
            self.symbolizer.find(state.pc).map(|symbol| {
                trace!(target: "symbols", "Found matching symbols @ PC: {}", symbol.join(", "));
            });

            trace!("Instruction: {:?}", instruction);

            if self.is_thumb() {
                trace!(target: "cpu", "Opcode: {:04X} | {:016b}", state.opcode as u16, state.opcode as u16);
            } else {
                trace!(target: "cpu", "Opcode: {:08X} | {:032b}", state.opcode, state.opcode);
            }

            debug!(target: "cpu",
                "[{:08X}] {:08X}: {: <50} [{}]",
                state.opcode,
                state.pc,
                format!("{}", instruction),
                self.compact_registers()
            );

            // clear the last read/write addresses
            self.mmio.last_rw_addr.clear();

            match instruction.opcode {
                Opcode::B | Opcode::Bl | Opcode::Bx => Handlers::branch(&instruction, self),
                Opcode::Push | Opcode::Pop => Handlers::push_pop(&instruction, self),
                Opcode::Cmp | Opcode::Tst | Opcode::Teq | Opcode::Cmn => Handlers::test(&instruction, self),
                Opcode::Mov | Opcode::Mvn => Handlers::move_data(&instruction, self),
                Opcode::Ldm | Opcode::Stm | Opcode::Ldr | Opcode::Str | Opcode::Swp => {
                    Handlers::load_store(&instruction, self)
                }
                Opcode::Mrs | Opcode::Msr => Handlers::psr_transfer(&instruction, self),
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
                | Opcode::Ror
                | Opcode::Mul
                | Opcode::Mla
                | Opcode::Umull
                | Opcode::Umlal
                | Opcode::Smull
                | Opcode::Smlal => Handlers::alu(&instruction, self),
                Opcode::Swi => Handlers::software_interrupt(&instruction, self),
            }

            trace!(target: "cpu", "\n{}", self);

            // do not increment PC if the pipeline has been flushed by an instruction
            if !self.pipeline.is_empty() {
                if self.is_thumb() {
                    self.registers.r[15] += 2;
                } else {
                    self.registers.r[15] += 4;
                }
            }

            return Ok((instruction, state));
        }

        if self.is_thumb() {
            self.registers.r[15] += 2;
        } else {
            self.registers.r[15] += 4;
        }

        Err(CpuError::NothingToDo)
    }

    pub fn skip_bios(&mut self) {
        // Initialize CPU state (post BIOS)
        self.set_processor_mode(ProcessorMode::Irq);
        self.write_register(&Register::R13, 0x03007fa0);
        self.set_processor_mode(ProcessorMode::Supervisor);
        self.write_register(&Register::R13, 0x03007fe0);
        self.set_processor_mode(ProcessorMode::User);
        self.write_register(&Register::R13, 0x03007f00);
        self.set_processor_mode(ProcessorMode::System);
        self.write_register(&Register::R13, 0x03007f00);
        self.write_register(&Register::R14, 0x08000000);
        self.write_register(&Register::R15, 0x08000000);
        self.mmio.io_postflg.write(0x01);
        self.mmio.openbus_bios = 0xE129F000; // initial openbus value after BIOS execution
        self.mmio.disable_bios_access();
    }

    fn compact_registers(&self) -> String {
        format!(
            "r0={:08X} r1={:08X} r2={:08X} r3={:08X} r4={:08X} r5={:08X} r6={:08X} r7={:08X} r8={:08X} r9={:08X} r10={:08X} r11={:08X} r12={:08X} sp={:08X} lr={:08X} pc={:08X} cpsr={} ime={} if={:016b} ie={:016b}",
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
            self.registers.cpsr,
            if *self.mmio.io_ime.value() != 0 { 1 } else { 0 },
            self.mmio.io_if.value(),
            self.mmio.io_ie.value(),
        )
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
                self.registers.r[15] = if self.is_thumb() { value & !0b1 } else { value };
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
                // TODO: only control
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
                // TODO: only flag and control
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

    pub fn push_stack(&mut self, value: u32) {
        let sp = self.get_sp();
        let addr = sp.wrapping_sub(4);
        self.mmio.write_u32(addr, value);
        self.write_register(&Register::R13, addr);
    }

    pub fn pop_stack(&mut self) -> u32 {
        let sp = self.get_sp();
        let value = self.mmio.read_u32(sp);
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
        trace!(target: "cpu", "Switched from {} to {}", current_mode, mode);
    }

    pub fn write_to_current_spsr(&mut self, value: Psr) {
        let mode = self.get_processor_mode();
        self.write_to_spsr(mode, value);
    }

    pub fn write_to_spsr(&mut self, mode: ProcessorMode, value: Psr) {
        if mode == ProcessorMode::User || mode == ProcessorMode::System {
            error!(target: "cpu", "Attempted to write to User/System SPSR");
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
                error!(target: "cpu", "Attempted to read from User/System SPSR");
                self.registers.cpsr
            }
            ProcessorMode::Fiq => self.registers.spsr[0],
            ProcessorMode::Supervisor => self.registers.spsr[1],
            ProcessorMode::Abort => self.registers.spsr[2],
            ProcessorMode::Irq => self.registers.spsr[3],
            ProcessorMode::Undefined => self.registers.spsr[4],
            _ => todo!(),
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
            " r0: {:08X}  r1: {:08X}  r2: {:08X}  r3: {:08X}\n",
            self.read_register(&Register::R0),
            self.read_register(&Register::R1),
            self.read_register(&Register::R2),
            self.read_register(&Register::R3)
        )?;
        write!(
            f,
            " r4: {:08X}  r5: {:08X}  r6: {:08X}  r7: {:08X}\n",
            self.read_register(&Register::R4),
            self.read_register(&Register::R5),
            self.read_register(&Register::R6),
            self.read_register(&Register::R7)
        )?;
        write!(
            f,
            " r8: {:08X}  r9: {:08X} r10: {:08X} r11: {:08X}\n",
            self.read_register(&Register::R8),
            self.read_register(&Register::R9),
            self.read_register(&Register::R10),
            self.read_register(&Register::R11)
        )?;
        write!(
            f,
            "r12: {:08X} r13: {:08X} r14: {:08X} r15: {:08X}\n",
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
            "spsr[0]: {}{{{},{}}}\nspsr[1]: {}{{{},{}}}\nspsr[2]: {}{{{},{}}}\nspsr[3]: {}{{{},{}}}\nspsr[4]: {}{{{},{}}}\n",
            self.registers.spsr[0],
            if self.registers.spsr[0].contains(Psr::T) {
                "Thumb"
            } else {
                "Arm"
            },
            self.registers.spsr[0].mode(),
            self.registers.spsr[1],
            if self.registers.spsr[1].contains(Psr::T) {
                "Thumb"
            } else {
                "Arm"
            },
            self.registers.spsr[1].mode(),
            self.registers.spsr[2],
            if self.registers.spsr[2].contains(Psr::T) {
                "Thumb"
            } else {
                "Arm"
            },
            self.registers.spsr[2].mode(),
            self.registers.spsr[3],
            if self.registers.spsr[3].contains(Psr::T) {
                "Thumb"
            } else {
                "Arm"
            },
            self.registers.spsr[3].mode(),
            self.registers.spsr[4],
            if self.registers.spsr[4].contains(Psr::T) {
                "Thumb"
            } else {
                "Arm"
            },
            self.registers.spsr[4].mode()
        )?;
        write!(
            f,
            "ime: {} if: {:016b} ie: {:016b}\n",
            if *self.mmio.io_ime.value() != 0 { 1 } else { 0 },
            self.mmio.io_if.value(),
            self.mmio.io_ie.value()
        )?;
        write!(
            f,
            "halt_cnt: {:08b} disp_stat: {:08b}\n",
            self.mmio.io_halt_cnt.value(),
            self.mmio.ppu.disp_stat.value()
        )?;
        write!(f, "{}", self.mmio.dma)
    }
}
