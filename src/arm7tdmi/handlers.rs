use super::cpu::Cpu;
use super::decoder::{Condition, Instruction, Opcode, Operand, ShiftSource, ShiftType};
use super::registers::Psr;
use crate::arm7tdmi::decoder::{Direction, Indexing, Register, TransferLength};
use crate::arm7tdmi::mode::ProcessorMode;
use crate::memory::mmio::Mmio;
use log::trace;

macro_rules! check_condition {
    ($cpu:expr, $instr:expr) => {
        if !Handlers::check_condition($cpu, &$instr.condition) {
            trace!("Skipping instruction due to condition");
            return;
        }
    };
}

pub struct Handlers {}

#[allow(unused_variables)]
impl Handlers {
    pub fn branch(instr: &Instruction, cpu: &mut Cpu, mmio: &mut Mmio) {
        check_condition!(cpu, instr);

        match instr {
            Instruction {
                opcode: Opcode::B,
                operand1: Some(Operand::Offset(offset)),
                ..
            } => {
                let pc = cpu.get_pc();
                let dst = pc.wrapping_add_signed(*offset);
                cpu.registers.r[15] = dst;
            }
            Instruction {
                opcode: Opcode::Bl,
                operand1: Some(Operand::Offset(offset)),
                ..
            } => {
                let pc = cpu.get_pc();
                let dst = pc.wrapping_add_signed(*offset);
                // the pipeline is 2 instructions ahead
                // but we want to store the address of the next instruction
                // a BL in thumb is split into two instructions, but we process it as one
                // that means PC points to the instruction after the 2nd half word of BL
                cpu.write_register(&Register::R14, if cpu.is_thumb() { pc | 1 } else { pc - 4 });
                cpu.registers.r[15] = dst;
            }
            Instruction {
                opcode: Opcode::Bx,
                operand1: Some(Operand::Register(register, None)),
                ..
            } => {
                let address = cpu.read_register(register);
                cpu.registers.cpsr.set(Psr::T, (address & 1) != 0);
                cpu.registers.r[15] = address & !1; // mask off last bit
            }
            _ => todo!("{:?}", instr),
        }

        cpu.pipeline.flush();
    }

    pub fn software_interrupt(instr: &Instruction, cpu: &mut Cpu, mmio: &mut Mmio) {
        check_condition!(cpu, instr);

        match instr {
            Instruction {
                opcode: Opcode::Swi,
                operand1: Some(Operand::Immediate(value, None)),
                ..
            } => {
                let pc = cpu.get_pc();
                cpu.write_register(&Register::R14, pc - 4);
                cpu.registers.r[15] = 0x08;

                // copy the current cpsr to spsr[current_mode]
                cpu.write_register(&Register::Spsr, cpu.read_register(&Register::Cpsr));

                // set the current mode to supervisor
                cpu.set_processor_mode(ProcessorMode::Supervisor);
            }
            _ => todo!("{:?}", instr),
        }

        cpu.pipeline.flush();
    }

    pub fn push_pop(instr: &Instruction, cpu: &mut Cpu, mmio: &mut Mmio) {
        check_condition!(cpu, instr);

        match instr {
            Instruction {
                opcode: Opcode::Push,
                operand1: Some(Operand::RegisterList(registers)),
                ..
            } => {
                for register in registers.iter().rev() {
                    cpu.push_stack(mmio, cpu.read_register(register));
                }
            }
            Instruction {
                opcode: Opcode::Pop,
                operand1: Some(Operand::RegisterList(registers)),
                ..
            } => {
                for register in registers {
                    let value = cpu.pop_stack(mmio);
                    cpu.write_register(register, value);
                }
            }
            _ => todo!("{:?}", instr),
        }
    }

    pub fn test(instr: &Instruction, cpu: &mut Cpu, mmio: &mut Mmio) {
        check_condition!(cpu, instr);

        let copy_spsr_to_cpsr_if_necessary = |cpu: &mut Cpu, rd: &Register| {
            // When Rd is R15 and the S flag is set the result of the operation
            // is placed in R15 and the SPSR corresponding to the
            // current mode is moved to the CPSR. This allows state
            // changes which atomically restore both PC and CPSR. This
            // form of instruction should not be used in User mode.

            if *rd == Register::R15 {
                let spsr = cpu.read_register(&Register::Spsr);
                cpu.write_register(&Register::Cpsr, spsr);
            }
        };

        match instr {
            Instruction {
                opcode: Opcode::Cmp | Opcode::Cmn,
                operand1: Some(Operand::Register(lhs, None)),
                operand2: Some(rhs),
                ..
            } => {
                let x = cpu.read_register(lhs);
                let y = Handlers::resolve_operand(rhs, cpu, false);
                let (result, carry) = match instr.opcode {
                    Opcode::Cmp => x.overflowing_sub(y),
                    Opcode::Cmn => x.overflowing_add(y),
                    _ => unreachable!(),
                };
                cpu.update_flag(Psr::N, (result as i32) < 0);
                cpu.update_flag(Psr::Z, result == 0);
                cpu.update_flag(Psr::C, !carry);
                cpu.update_flag(Psr::V, ((x ^ y) & (x ^ result) & 0x8000_0000) != 0);

                copy_spsr_to_cpsr_if_necessary(cpu, lhs);
            }
            Instruction {
                opcode: Opcode::Teq,
                operand1: Some(Operand::Register(lhs, None)),
                operand2: Some(rhs),
                ..
            } => {
                let x = cpu.read_register(lhs);
                let y = Handlers::resolve_operand(rhs, cpu, true);
                let result = x ^ y;
                cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                cpu.update_flag(Psr::Z, result == 0);

                copy_spsr_to_cpsr_if_necessary(cpu, lhs);
            }
            Instruction {
                opcode: Opcode::Tst,
                operand1: Some(Operand::Register(lhs, None)),
                operand2: Some(rhs),
                ..
            } => {
                let x = cpu.read_register(lhs);
                let y = Handlers::resolve_operand(rhs, cpu, true);
                let result = x & y;
                cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                cpu.update_flag(Psr::Z, result == 0);

                copy_spsr_to_cpsr_if_necessary(cpu, lhs);
            }
            _ => todo!("{:?}", instr),
        }
    }

    pub fn move_data(instr: &Instruction, cpu: &mut Cpu, mmio: &mut Mmio) {
        check_condition!(cpu, instr);

        match instr {
            Instruction {
                opcode: Opcode::Mov,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(src),
                set_condition_flags,
                ..
            } => {
                let value = Handlers::resolve_operand(src, cpu, *set_condition_flags);
                let extra_fetch = if src.is_register(&Register::R15)
                    && let Some(ShiftSource::Register(_)) = Handlers::try_fetch_shifted_operand(src)
                {
                    4
                } else {
                    0
                };
                let result = value + extra_fetch;
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                }
            }
            Instruction {
                opcode: Opcode::Mvn,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(src),
                set_condition_flags,
                ..
            } => {
                let value = Handlers::resolve_operand(src, cpu, *set_condition_flags);
                let extra_fetch = if src.is_register(&Register::R15)
                    && let Some(ShiftSource::Register(_)) = Handlers::try_fetch_shifted_operand(src)
                {
                    4
                } else {
                    0
                };
                let result = !(value + extra_fetch);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                }
            }
            _ => todo!("{:?}", instr),
        }
    }

    pub fn load_store(instr: &Instruction, cpu: &mut Cpu, mmio: &mut Mmio) {
        check_condition!(cpu, instr);

        match instr {
            Instruction {
                opcode: Opcode::Ldr,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: Some(step),
                transfer_length: Some(length),
                offset_direction: Some(operation),
                set_condition_flags,
                indexing: Some(indexing),
                writeback,
                ..
            } => {
                let mut address = if *src == Register::R15 && cpu.is_thumb() {
                    // WhenGryphonsFly — Today at 1:51 PM
                    // In thumb mode, PC-relative loads treat bit 1 of PC as always 0
                    cpu.read_register(src) & !0b10
                } else {
                    cpu.read_register(src)
                };
                let step = Handlers::resolve_operand(step, cpu, *set_condition_flags);

                if *indexing == Indexing::Pre {
                    if *operation == Direction::Up {
                        address = address.wrapping_add(step)
                    } else {
                        address = address.wrapping_sub(step)
                    }
                };

                match length {
                    TransferLength::Byte => {
                        let value = mmio.read(address);
                        cpu.write_register_u8(dst, value);
                        if *set_condition_flags {
                            cpu.update_flag(Psr::N, value & 0x80 != 0);
                            cpu.update_flag(Psr::Z, value == 0);
                        }
                    }
                    TransferLength::HalfWord => {
                        let value = mmio.read_u16(address);
                        cpu.write_register_u16(dst, value);
                        if *set_condition_flags {
                            cpu.update_flag(Psr::N, value & 0x8000 != 0);
                            cpu.update_flag(Psr::Z, value == 0);
                        }
                    }
                    TransferLength::Word => {
                        let value = mmio.read_u32(address);
                        cpu.write_register(dst, value);
                        if *set_condition_flags {
                            cpu.update_flag(Psr::N, value & 0x8000_0000 != 0);
                            cpu.update_flag(Psr::Z, value == 0);
                        }
                    }
                }

                if *indexing == Indexing::Post {
                    if *operation == Direction::Up {
                        address = address.wrapping_add(step);
                    } else {
                        address = address.wrapping_sub(step);
                    }
                }

                if *writeback {
                    cpu.write_register(src, address);
                }
            }
            Instruction {
                opcode: Opcode::Str,
                operand1: Some(Operand::Register(src, None)),
                operand2: Some(Operand::Register(dst, None)),
                operand3: Some(step),
                transfer_length: Some(length),
                offset_direction: Some(operation),
                set_condition_flags,
                indexing: Some(indexing),
                writeback,
                ..
            } => {
                let mut address = cpu.read_register(dst);
                let step = Handlers::resolve_operand(step, cpu, *set_condition_flags);

                if *indexing == Indexing::Pre {
                    if *operation == Direction::Up {
                        address = address.wrapping_add(step)
                    } else {
                        address = address.wrapping_sub(step)
                    }
                };

                match length {
                    TransferLength::Byte => {
                        let value = cpu.read_register(src) as u8;
                        mmio.write(address, value);
                        if *set_condition_flags {
                            cpu.update_flag(Psr::N, value & 0x80 != 0);
                            cpu.update_flag(Psr::Z, value == 0);
                        }
                    }
                    TransferLength::HalfWord => {
                        let value = cpu.read_register(src) as u16;
                        mmio.write_u16(address, value);
                        if *set_condition_flags {
                            cpu.update_flag(Psr::N, value & 0x8000 != 0);
                            cpu.update_flag(Psr::Z, value == 0);
                        }
                    }
                    TransferLength::Word => {
                        let value = cpu.read_register(src);
                        mmio.write_u32(address, value);
                        if *set_condition_flags {
                            cpu.update_flag(Psr::N, value & 0x8000_0000 != 0);
                            cpu.update_flag(Psr::Z, value == 0);
                        }
                    }
                }

                if *indexing == Indexing::Post {
                    if *operation == Direction::Up {
                        address = address.wrapping_add(step);
                    } else {
                        address = address.wrapping_sub(step);
                    }
                }

                if *writeback {
                    cpu.write_register(dst, address);
                }
            }
            Instruction {
                opcode: Opcode::Ldm,
                operand1: Some(Operand::Register(src_base, None)),
                operand2: Some(Operand::RegisterList(registers)),
                offset_direction: Some(operation),
                indexing: Some(indexing),
                writeback,
                ..
            } => {
                let mut address = cpu.read_register(src_base);

                for register in registers {
                    if *indexing == Indexing::Pre {
                        if *operation == Direction::Up {
                            address += 4;
                        } else {
                            address -= 4;
                        }
                    }

                    let value = mmio.read_u32(address);
                    cpu.write_register(register, value);

                    if *indexing == Indexing::Post {
                        if *operation == Direction::Up {
                            address += 4;
                        } else {
                            address -= 4;
                        }
                    }
                }

                if *writeback {
                    cpu.write_register(src_base, address);
                }
            }
            Instruction {
                opcode: Opcode::Stm,
                operand1: Some(Operand::Register(dst_base, None)),
                operand2: Some(Operand::RegisterList(registers)),
                offset_direction: Some(operation),
                indexing: Some(indexing),
                writeback,
                ..
            } => {
                let mut address = cpu.read_register(dst_base);

                for register in registers {
                    if *indexing == Indexing::Pre {
                        if *operation == Direction::Up {
                            address += 4;
                        } else {
                            address -= 4;
                        }
                    }

                    let value = cpu.read_register(register);
                    mmio.write_u32(address, value);

                    if *indexing == Indexing::Post {
                        if *operation == Direction::Up {
                            address += 4;
                        } else {
                            address -= 4;
                        }
                    }
                }

                if *writeback {
                    cpu.write_register(dst_base, address);
                }
            }
            _ => todo!("{:?}", instr),
        }
    }

    pub fn psr_transfer(instr: &Instruction, cpu: &mut Cpu, mmio: &mut Mmio) {
        check_condition!(cpu, instr);

        match instr {
            Instruction {
                opcode: Opcode::Msr | Opcode::Mrs,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(src),
                ..
            } => {
                let src = Handlers::resolve_operand(src, cpu, false);
                cpu.write_register(dst, src);
            }
            _ => todo!("{:?}", instr),
        }
    }

    pub fn alu(instr: &Instruction, cpu: &mut Cpu, mmio: &mut Mmio) {
        check_condition!(cpu, instr);

        let copy_spsr_to_cpsr_if_necessary = |cpu: &mut Cpu, rd: &Register| {
            // When Rd is R15 and the S flag is set the result of the operation
            // is placed in R15 and the SPSR corresponding to the
            // current mode is moved to the CPSR. This allows state
            // changes which atomically restore both PC and CPSR. This
            // form of instruction should not be used in User mode.

            if *rd == Register::R15 {
                let spsr = cpu.read_register(&Register::Spsr);
                cpu.write_register(&Register::Cpsr, spsr);
                cpu.pipeline.flush();
            }
        };

        match instr {
            Instruction {
                opcode: Opcode::Add,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(x),
                operand3: Some(y),
                set_condition_flags,
                ..
            } => {
                let extra_fetch = match (
                    Handlers::try_fetch_shifted_operand(x),
                    Handlers::try_fetch_shifted_operand(y),
                ) {
                    (_, Some(ShiftSource::Register(_))) => 4,
                    (Some(ShiftSource::Register(_)), _) => 4,
                    _ => 0,
                };
                let x = Handlers::resolve_operand(x, cpu, *set_condition_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_condition_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };

                let (result, carry) = x.overflowing_add(y);
                let (_, overflow) = (x as i32).overflowing_add(y as i32);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, carry);
                    cpu.update_flag(Psr::V, overflow);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Add,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(src),
                operand3: None,
                set_condition_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = Handlers::resolve_operand(src, cpu, *set_condition_flags);
                let (result, carry) = x.overflowing_add(y);
                let (_, overflow) = (x as i32).overflowing_add(y as i32);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, carry);
                    cpu.update_flag(Psr::V, overflow);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Adc,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(x),
                operand3: Some(y),
                set_condition_flags,
                ..
            } => {
                let carry = cpu.registers.cpsr.contains(Psr::C) as u32; // Grab carry first, as it may be modified due to shifter
                let x = Handlers::resolve_operand(x, cpu, *set_condition_flags);
                let y = Handlers::resolve_operand(y, cpu, *set_condition_flags);

                let (result, carry1) = x.overflowing_add(y);
                let (result, carry2) = result.overflowing_add(carry);

                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, carry1 || carry2);

                    let overflow = ((x ^ result) & (y ^ result) & 0x8000_0000) != 0;
                    cpu.update_flag(Psr::V, overflow);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Sub,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(x),
                operand3: Some(y),
                set_condition_flags,
                ..
            } => {
                let extra_fetch = match (
                    Handlers::try_fetch_shifted_operand(x),
                    Handlers::try_fetch_shifted_operand(y),
                ) {
                    (_, Some(ShiftSource::Register(_))) => 4,
                    (Some(ShiftSource::Register(_)), _) => 4,
                    _ => 0,
                };
                let x = Handlers::resolve_operand(x, cpu, *set_condition_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_condition_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };
                let (result, borrow) = x.overflowing_sub(y);
                let (_, overflow) = (x as i32).overflowing_sub(y as i32);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, !borrow);
                    cpu.update_flag(Psr::V, overflow);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Sub,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(src),
                operand3: None,
                set_condition_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = Handlers::resolve_operand(src, cpu, *set_condition_flags);
                let (result, borrow) = x.overflowing_sub(y);
                let (_, overflow) = (x as i32).overflowing_sub(y as i32);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, !borrow);
                    cpu.update_flag(Psr::V, overflow);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Sbc,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(x),
                operand3: Some(y),
                set_condition_flags,
                ..
            } => {
                let extra_fetch = match (
                    Handlers::try_fetch_shifted_operand(x),
                    Handlers::try_fetch_shifted_operand(y),
                ) {
                    (_, Some(ShiftSource::Register(_))) => 4,
                    (Some(ShiftSource::Register(_)), _) => 4,
                    _ => 0,
                };
                let x = Handlers::resolve_operand(x, cpu, *set_condition_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_condition_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };
                let carry = cpu.registers.cpsr.contains(Psr::C) as u32;

                let (result, borrow1) = x.overflowing_sub(y);
                let (result, borrow2) = result.overflowing_sub(1 - carry);

                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, !borrow1 && !borrow2);

                    let overflow = ((x ^ y) & (x ^ result) & 0x8000_0000) != 0;
                    cpu.update_flag(Psr::V, overflow);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Sbc,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(src),
                operand3: None,
                set_condition_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = Handlers::resolve_operand(src, cpu, *set_condition_flags);
                let carry = cpu.registers.cpsr.contains(Psr::C) as u32;

                let (result, borrow1) = x.overflowing_sub(y);
                let (result, borrow2) = result.overflowing_sub(1 - carry);

                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, !borrow1 && !borrow2);

                    let overflow = ((x ^ y) & (x ^ result) & 0x8000_0000) != 0;
                    cpu.update_flag(Psr::V, overflow);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::And,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(x),
                operand3: Some(y),
                set_condition_flags,
                ..
            } => {
                let extra_fetch = match (
                    Handlers::try_fetch_shifted_operand(x),
                    Handlers::try_fetch_shifted_operand(y),
                ) {
                    (_, Some(ShiftSource::Register(_))) => 4,
                    (Some(ShiftSource::Register(_)), _) => 4,
                    _ => 0,
                };
                let x = Handlers::resolve_operand(x, cpu, *set_condition_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_condition_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };
                let result = x & y;
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::And,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: None,
                set_condition_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = cpu.read_register(src);
                let result = x & y;
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Orr,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(x),
                operand3: Some(y),
                set_condition_flags,
                ..
            } => {
                let extra_fetch = match (
                    Handlers::try_fetch_shifted_operand(x),
                    Handlers::try_fetch_shifted_operand(y),
                ) {
                    (_, Some(ShiftSource::Register(_))) => 4,
                    (Some(ShiftSource::Register(_)), _) => 4,
                    _ => 0,
                };
                let x = Handlers::resolve_operand(x, cpu, *set_condition_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_condition_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };
                let result = x | y;
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Orr,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: None,
                set_condition_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = cpu.read_register(src);
                let result = x | y;
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Eor,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(x),
                operand3: Some(y),
                set_condition_flags,
                ..
            } => {
                let extra_fetch = match (
                    Handlers::try_fetch_shifted_operand(x),
                    Handlers::try_fetch_shifted_operand(y),
                ) {
                    (_, Some(ShiftSource::Register(_))) => 4,
                    (Some(ShiftSource::Register(_)), _) => 4,
                    _ => 0,
                };
                let x = Handlers::resolve_operand(x, cpu, *set_condition_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_condition_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };
                let result = x ^ y;
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Eor,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: None,
                set_condition_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = cpu.read_register(src);
                let result = x ^ y;
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Rsb,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(x),
                operand3: Some(y),
                set_condition_flags,
                ..
            } => {
                let extra_fetch = match (
                    Handlers::try_fetch_shifted_operand(x),
                    Handlers::try_fetch_shifted_operand(y),
                ) {
                    (_, Some(ShiftSource::Register(_))) => 4,
                    (Some(ShiftSource::Register(_)), _) => 4,
                    _ => 0,
                };
                let x = Handlers::resolve_operand(x, cpu, *set_condition_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_condition_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };
                let (result, borrow) = y.overflowing_sub(x);
                let (_, overflow) = (y as i32).overflowing_sub(x as i32);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, !borrow);
                    cpu.update_flag(Psr::V, overflow);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Rsc,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(x),
                operand3: Some(y),
                set_condition_flags,
                ..
            } => {
                let extra_fetch = match (
                    Handlers::try_fetch_shifted_operand(x),
                    Handlers::try_fetch_shifted_operand(y),
                ) {
                    (_, Some(ShiftSource::Register(_))) => 4,
                    (Some(ShiftSource::Register(_)), _) => 4,
                    _ => 0,
                };
                let x = Handlers::resolve_operand(x, cpu, *set_condition_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_condition_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };
                let carry = cpu.registers.cpsr.contains(Psr::C) as u32;

                let (result, borrow1) = y.overflowing_sub(x);
                let (result, borrow2) = result.overflowing_sub(1 - carry);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, !borrow1 && !borrow2);

                    let overflow = ((x ^ y) & (x ^ result) & 0x8000_0000) != 0;
                    cpu.update_flag(Psr::V, overflow);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Neg,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: None,
                set_condition_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = cpu.read_register(src);
                let (result, borrow) = y.overflowing_sub(x);
                let (_, overflow) = (y as i32).overflowing_sub(x as i32);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, !borrow);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Bic,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: None,
                set_condition_flags,
                ..
            } => {
                let src = cpu.read_register(src);
                let result = src & !cpu.read_register(dst);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Bic,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: Some(Operand::Immediate(pos, None)),
                set_condition_flags,
                ..
            } => {
                let src = cpu.read_register(src);
                let result = src & !pos;
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Lsl,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: Some(Operand::Immediate(shift, None)),
                set_condition_flags,
                ..
            } => {
                let value = cpu.read_register(src);
                let result = value.wrapping_shl(*shift);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, value & (1 << (32 - *shift)) != 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Lsr,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: Some(Operand::Immediate(shift, None)),
                set_condition_flags,
                ..
            } => {
                let value = cpu.read_register(src);
                let result = value.wrapping_shr(*shift);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, value & (1 << (*shift - 1)) != 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Asr,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: Some(Operand::Immediate(shift, None)),
                set_condition_flags,
                ..
            } => {
                let value = cpu.read_register(src);
                let result = value.wrapping_shr(*shift);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, value & (1 << (*shift - 1)) != 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Mul,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(lhs, None)),
                operand3: Some(Operand::Register(rhs, None)),
                operand4: None,
                set_condition_flags,
                ..
            } => {
                let lhs = cpu.read_register(lhs);
                let rhs = cpu.read_register(rhs);
                let result = lhs.wrapping_mul(rhs);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Mla,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(lhs, None)),
                operand3: Some(Operand::Register(rhs, None)),
                operand4: Some(Operand::Register(acc, None)),
                set_condition_flags,
                ..
            } => {
                let lhs = cpu.read_register(lhs);
                let rhs = cpu.read_register(rhs);
                let acc = cpu.read_register(acc);
                let result = lhs.wrapping_mul(rhs).wrapping_add(acc);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                }
            }
            _ => todo!("{:?}", instr),
        }
    }

    fn resolve_operand(operand: &Operand, cpu: &mut Cpu, set_condition_flags: bool) -> u32 {
        match operand {
            Operand::Immediate(value, Some(shift)) => Handlers::process_shift(*value, shift, cpu, set_condition_flags),
            Operand::Immediate(value, None) => *value,
            Operand::Register(register, Some(shift)) => {
                Handlers::process_shift(cpu.read_register(register), shift, cpu, set_condition_flags)
            }
            Operand::Register(register, None) => cpu.read_register(register),
            _ => unreachable!(),
        }
    }

    fn unwrap_shift_source(cpu: &Cpu, src: &ShiftSource) -> u32 {
        match src {
            ShiftSource::Immediate(value) => *value,
            ShiftSource::Register(register) => cpu.read_register(register) & 0xff,
        }
    }

    fn process_shift(value: u32, shift: &ShiftType, cpu: &mut Cpu, set_condition_flags: bool) -> u32 {
        match shift {
            ShiftType::LogicalLeft(src) => {
                let shift = Handlers::unwrap_shift_source(cpu, src);
                let result = value.checked_shl(shift).unwrap_or(0);
                if set_condition_flags {
                    match shift {
                        0 => {}
                        1..=31 => {
                            // not handling shift == 0 because it would create a mask of 0
                            // and the result would be 0
                            let mask = 1 << (32 - shift);
                            let carry_out = value & mask != 0;
                            cpu.update_flag(Psr::C, carry_out)
                        }
                        32 => cpu.update_flag(Psr::C, value & 1 != 0),
                        _ => cpu.update_flag(Psr::C, false),
                    }
                }
                result
            }
            ShiftType::LogicalRight(src) => {
                let shift = Handlers::unwrap_shift_source(cpu, src);
                let result = value.checked_shr(shift).unwrap_or(0);
                if set_condition_flags {
                    match shift {
                        0 => {}
                        1..=31 => {
                            // not handling shift == 0 because it would create a mask of 0
                            // and the result would be 0
                            let mask = 1 << (shift - 1);
                            let carry_out = value & mask != 0;
                            cpu.update_flag(Psr::C, carry_out)
                        }
                        32 => cpu.update_flag(Psr::C, value & (1 << 31) != 0),
                        _ => cpu.update_flag(Psr::C, false),
                    }
                }
                result
            }
            ShiftType::ArithmeticRight(src) => {
                let shift = Handlers::unwrap_shift_source(cpu, src);
                let result = if shift >= 32 {
                    if value & (1 << 31) != 0 {
                        0xffffffff
                    } else {
                        0x00000000
                    }
                } else {
                    let shifted = (value as i32) >> shift;
                    shifted as u32
                };

                if set_condition_flags {
                    match shift {
                        0 => {}
                        1..=31 => {
                            // not handling shift == 0 because it would create a mask of 0
                            // and the result would be 0
                            let mask = 1 << (shift - 1);
                            let carry_out = value & mask != 0;
                            cpu.update_flag(Psr::C, carry_out)
                        }
                        32 => cpu.update_flag(Psr::C, value & (1 << 31) != 0),
                        _ => cpu.update_flag(Psr::C, false),
                    }
                }
                result
            }
            ShiftType::RotateRight(src) => {
                let shift = Handlers::unwrap_shift_source(cpu, src);
                let result = value.rotate_right(shift);
                if set_condition_flags {
                    match shift {
                        0 => {}
                        1..=31 => {
                            // not handling shift == 0 because it would create a mask of 0
                            // and the result would be 0
                            let mask = 1 << (shift - 1);
                            let carry_out = value & mask != 0;
                            cpu.update_flag(Psr::C, carry_out)
                        }
                        32 => cpu.update_flag(Psr::C, value & (1 << 31) != 0),
                        _ => cpu.update_flag(Psr::C, false),
                    }
                }
                result
            }
            ShiftType::RotateRightExtended => {
                let new_carry = (value & 1) != 0;
                let result = (value >> 1) | ((cpu.registers.cpsr.contains(Psr::C) as u32) << 31);
                if set_condition_flags {
                    cpu.update_flag(Psr::C, new_carry);
                }
                result
            }
        }
    }

    fn try_fetch_shifted_operand(operand: &Operand) -> Option<ShiftSource> {
        match operand {
            Operand::Register(_, Some(ShiftType::LogicalLeft(src))) => Some(*src),
            Operand::Register(_, Some(ShiftType::LogicalRight(src))) => Some(*src),
            Operand::Register(_, Some(ShiftType::ArithmeticRight(src))) => Some(*src),
            Operand::Register(_, Some(ShiftType::RotateRight(src))) => Some(*src),
            _ => None,
        }
    }

    fn check_condition(cpu: &Cpu, condition: &Condition) -> bool {
        match condition {
            Condition::Always => true,
            Condition::Equal => cpu.registers.cpsr.contains(Psr::Z), // Z == 1
            Condition::NotEqual => !cpu.registers.cpsr.contains(Psr::Z), // Z == 0
            Condition::UnsignedHigherOrSame => cpu.registers.cpsr.contains(Psr::C), // C == 1
            Condition::UnsignedLower => !cpu.registers.cpsr.contains(Psr::C), // C == 0
            Condition::Negative => cpu.registers.cpsr.contains(Psr::N), // N == 1
            Condition::PositiveOrZero => !cpu.registers.cpsr.contains(Psr::N), // N == 0
            Condition::Overflow => cpu.registers.cpsr.contains(Psr::V), // V == 1
            Condition::NoOverflow => !cpu.registers.cpsr.contains(Psr::V), // V == 0
            Condition::UnsignedHigher => cpu.registers.cpsr.contains(Psr::C) && !cpu.registers.cpsr.contains(Psr::Z), // C == 1 and Z == 0
            Condition::UnsignedLowerOrSame => {
                !cpu.registers.cpsr.contains(Psr::C) || cpu.registers.cpsr.contains(Psr::Z)
            } // C == 0 or Z == 1
            Condition::GreaterOrEqual => cpu.registers.cpsr.contains(Psr::N) == cpu.registers.cpsr.contains(Psr::V), // N == V
            Condition::LessThan => cpu.registers.cpsr.contains(Psr::N) != cpu.registers.cpsr.contains(Psr::V), // N != V
            Condition::GreaterThan => {
                !cpu.registers.cpsr.contains(Psr::Z)
                    && (cpu.registers.cpsr.contains(Psr::N) == cpu.registers.cpsr.contains(Psr::V))
            } // Z == 0 and N == V
            Condition::LessThanOrEqual => {
                cpu.registers.cpsr.contains(Psr::Z)
                    || (cpu.registers.cpsr.contains(Psr::N) != cpu.registers.cpsr.contains(Psr::V))
            } // Z == 1 or N != V
        }
    }
}
