use super::{
    cpu::Cpu,
    decoder::{Condition, Instruction, Opcode, Operand, ShiftType},
    registers::Psr,
};
use crate::{
    arm7tdmi::{
        cpu::ProcessorMode,
        decoder::{Direction, Indexing, Register, TransferLength},
    },
    memory::mmio::Mmio,
};
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
                cpu.registers.r[14] = if cpu.is_thumb() { pc | 1 } else { pc - 4 };
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
                cpu.registers.r[14] = pc - 4;
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

        match instr {
            Instruction {
                opcode: Opcode::Cmp,
                operand1: Some(Operand::Register(lhs, None)),
                operand2: Some(rhs),
                ..
            } => {
                let lhs = cpu.read_register(lhs);
                let rhs = Handlers::resolve_operand(rhs, cpu);
                let (result, carry) = lhs.overflowing_sub(rhs);
                cpu.update_flag(Psr::N, (result as i32) < 0);
                cpu.update_flag(Psr::Z, result == 0);
                cpu.update_flag(Psr::C, !carry);
                cpu.update_flag(Psr::V, ((lhs ^ rhs) & (lhs ^ result) & 0x8000_0000) != 0);
            }
            Instruction {
                opcode: Opcode::Teq,
                operand1: Some(Operand::Register(lhs, None)),
                operand2: Some(rhs),
                ..
            } => {
                let lhs = cpu.read_register(lhs);
                let rhs = Handlers::resolve_operand(rhs, cpu);
                let result = lhs ^ rhs;
                cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                cpu.update_flag(Psr::Z, result == 0);
            }
            Instruction {
                opcode: Opcode::Tst,
                operand1: Some(Operand::Register(lhs, None)),
                operand2: Some(rhs),
                ..
            } => {
                let lhs = cpu.read_register(lhs);
                let rhs = Handlers::resolve_operand(rhs, cpu);
                let result = lhs & rhs;
                cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                cpu.update_flag(Psr::Z, result == 0);
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
                let value = Handlers::resolve_operand(src, cpu);
                cpu.write_register(dst, value);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, value & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, value == 0);
                }
            }
            Instruction {
                opcode: Opcode::Mvn,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(src),
                set_condition_flags,
                ..
            } => {
                let value = !Handlers::resolve_operand(src, cpu); // bitwise not
                cpu.write_register(dst, value);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, value & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, value == 0);
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
                let mut address = cpu.read_register(src);
                let step = Handlers::resolve_operand(step, cpu);

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
                let step = Handlers::resolve_operand(step, cpu);

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
                    cpu.write_register(dst_base, address);
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
                let src = Handlers::resolve_operand(src, cpu);
                cpu.write_register(dst, src);
            }
            _ => todo!("{:?}", instr),
        }
    }

    pub fn alu(instr: &Instruction, cpu: &mut Cpu, mmio: &mut Mmio) {
        check_condition!(cpu, instr);

        match instr {
            Instruction {
                opcode: Opcode::Add,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(x),
                operand3: Some(y),
                set_condition_flags,
                ..
            } => {
                let x = Handlers::resolve_operand(x, cpu);
                let y = Handlers::resolve_operand(y, cpu);
                let (result, carry) = x.overflowing_add(y);
                let (_, overflow) = (x as i32).overflowing_add(y as i32);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, carry);
                    cpu.update_flag(Psr::V, overflow);
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
                let y = Handlers::resolve_operand(src, cpu);
                let (result, carry) = x.overflowing_add(y);
                let (_, overflow) = (x as i32).overflowing_add(y as i32);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, carry);
                    cpu.update_flag(Psr::V, overflow);
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
                let x = Handlers::resolve_operand(x, cpu);
                let y = Handlers::resolve_operand(y, cpu);
                let carry = cpu.registers.cpsr.contains(Psr::C) as u32;

                let (result, carry1) = x.overflowing_add(y);
                let (result, carry2) = result.overflowing_add(carry);

                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, carry1 || carry2);

                    let overflow = ((x ^ result) & (y ^ result) & 0x8000_0000) != 0;
                    cpu.update_flag(Psr::V, overflow);
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
                let x = Handlers::resolve_operand(x, cpu);
                let y = Handlers::resolve_operand(y, cpu);
                let (result, borrow) = x.overflowing_sub(y);
                let (_, overflow) = (x as i32).overflowing_sub(y as i32);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, !borrow);
                    cpu.update_flag(Psr::V, overflow);
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
                let y = Handlers::resolve_operand(src, cpu);
                let (result, borrow) = x.overflowing_sub(y);
                let (_, overflow) = (x as i32).overflowing_sub(y as i32);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, !borrow);
                    cpu.update_flag(Psr::V, overflow);
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
                let x = Handlers::resolve_operand(x, cpu);
                let y = Handlers::resolve_operand(y, cpu);
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
                let y = Handlers::resolve_operand(src, cpu);
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
                let x = Handlers::resolve_operand(x, cpu);
                let y = Handlers::resolve_operand(y, cpu);
                let result = x & y;
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
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
                let x = Handlers::resolve_operand(x, cpu);
                let y = Handlers::resolve_operand(y, cpu);
                let result = x | y;
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
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
                let x = Handlers::resolve_operand(x, cpu);
                let y = Handlers::resolve_operand(y, cpu);
                let result = x ^ y;
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
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
                let x = Handlers::resolve_operand(x, cpu);
                let y = Handlers::resolve_operand(y, cpu);
                let (result, borrow) = y.overflowing_sub(x);
                let (_, overflow) = (y as i32).overflowing_sub(x as i32);
                cpu.write_register(dst, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, !borrow);
                    cpu.update_flag(Psr::V, overflow);
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
                let x = Handlers::resolve_operand(x, cpu);
                let y = Handlers::resolve_operand(y, cpu);
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
                }
            }
            Instruction {
                opcode: Opcode::Bic,
                operand1: Some(Operand::Register(lhs, None)),
                operand2: Some(Operand::Register(rhs, None)),
                set_condition_flags,
                ..
            } => {
                let x = cpu.read_register(lhs);
                let y = cpu.read_register(rhs);
                let result = x & !y;
                cpu.write_register(lhs, result);

                if *set_condition_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
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
                }
            }
            _ => todo!("{:?}", instr),
        }
    }

    fn resolve_operand(operand: &Operand, cpu: &Cpu) -> u32 {
        match operand {
            Operand::Immediate(value, Some(shift)) => Handlers::process_shift(*value, shift),
            Operand::Immediate(value, None) => *value,
            Operand::Register(register, Some(shift)) => {
                Handlers::process_shift(cpu.read_register(register), shift)
            }
            Operand::Register(register, None) => cpu.read_register(register),
            _ => unreachable!(),
        }
    }

    fn process_shift(value: u32, shift: &ShiftType) -> u32 {
        match shift {
            ShiftType::LogicalLeft(shift) => value.wrapping_shl(*shift),
            ShiftType::LogicalRight(shift) => value.wrapping_shr(*shift),
            ShiftType::ArithmeticRight(shift) => value.wrapping_shr(*shift),
            ShiftType::RotateRight(shift) => value.rotate_right(*shift),
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
            Condition::UnsignedHigher => {
                cpu.registers.cpsr.contains(Psr::C) && !cpu.registers.cpsr.contains(Psr::Z)
            } // C == 1 and Z == 0
            Condition::UnsignedLowerOrSame => {
                !cpu.registers.cpsr.contains(Psr::C) || cpu.registers.cpsr.contains(Psr::Z)
            } // C == 0 or Z == 1
            Condition::GreaterOrEqual => {
                cpu.registers.cpsr.contains(Psr::N) == cpu.registers.cpsr.contains(Psr::V)
            } // N == V
            Condition::LessThan => {
                cpu.registers.cpsr.contains(Psr::N) != cpu.registers.cpsr.contains(Psr::V)
            } // N != V
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
