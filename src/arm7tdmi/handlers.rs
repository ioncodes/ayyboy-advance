use crate::memory::mmio::Mmio;

use super::{
    cpu::Cpu,
    decoder::{Condition, Instruction, Opcode, Operand, ShiftType},
    registers::Cpsr,
};

macro_rules! check_condition {
    ($cpu:expr, $instr:expr) => {
        if !Handlers::check_condition($cpu, &$instr.condition) {
            return;
        }
    };
}

pub struct Handlers {}

impl Handlers {
    fn check_condition(cpu: &Cpu, condition: &Condition) -> bool {
        match condition {
            Condition::Always => true,
            Condition::Equal => cpu.registers.cpsr.contains(Cpsr::Z), // Z == 1
            Condition::NotEqual => !cpu.registers.cpsr.contains(Cpsr::Z), // Z == 0
            Condition::UnsignedHigherOrSame => cpu.registers.cpsr.contains(Cpsr::C), // C == 1
            Condition::UnsignedLower => !cpu.registers.cpsr.contains(Cpsr::C), // C == 0
            Condition::Negative => cpu.registers.cpsr.contains(Cpsr::N), // N == 1
            Condition::PositiveOrZero => !cpu.registers.cpsr.contains(Cpsr::N), // N == 0
            Condition::Overflow => cpu.registers.cpsr.contains(Cpsr::V), // V == 1
            Condition::NoOverflow => !cpu.registers.cpsr.contains(Cpsr::V), // V == 0
            Condition::UnsignedHigher => {
                cpu.registers.cpsr.contains(Cpsr::C) && !cpu.registers.cpsr.contains(Cpsr::Z)
            } // C == 1 and Z == 0
            Condition::UnsignedLowerOrSame => {
                !cpu.registers.cpsr.contains(Cpsr::C) || cpu.registers.cpsr.contains(Cpsr::Z)
            } // C == 0 or Z == 1
            Condition::GreaterOrEqual => {
                cpu.registers.cpsr.contains(Cpsr::N) == cpu.registers.cpsr.contains(Cpsr::V)
            } // N == V
            Condition::LessThan => {
                cpu.registers.cpsr.contains(Cpsr::N) != cpu.registers.cpsr.contains(Cpsr::V)
            } // N != V
            Condition::GreaterThan => {
                !cpu.registers.cpsr.contains(Cpsr::Z)
                    && (cpu.registers.cpsr.contains(Cpsr::N)
                        == cpu.registers.cpsr.contains(Cpsr::V))
            } // Z == 0 and N == V
            Condition::LessThanOrEqual => {
                cpu.registers.cpsr.contains(Cpsr::Z)
                    || (cpu.registers.cpsr.contains(Cpsr::N)
                        != cpu.registers.cpsr.contains(Cpsr::V))
            } // Z == 1 or N != V
        }
    }

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
                cpu.registers.r[14] = pc;
                cpu.registers.r[15] = dst;
            }
            _ => todo!("{:?}", instr),
        }
    }

    pub fn push_pop(instr: &Instruction, cpu: &mut Cpu, mmio: &mut Mmio) {
        match instr {
            Instruction {
                opcode: Opcode::Push,
                operand1: Some(Operand::RegisterList(registers)),
                ..
            } => {
                for register in registers {
                    cpu.push_stack(mmio, cpu.read_register(register));
                }
            }
            Instruction {
                opcode: Opcode::Pop,
                operand1: Some(Operand::RegisterList(registers)),
                ..
            } => {
                for register in registers.iter().rev() {
                    let value = cpu.pop_stack(mmio);
                    cpu.write_register(register, value);
                }
            }
            _ => todo!("{:?}", instr),
        }
    }

    pub fn test(instr: &Instruction, cpu: &mut Cpu, mmio: &mut Mmio) {
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
                cpu.update_flag(Cpsr::N, result > lhs);
                cpu.update_flag(Cpsr::Z, lhs == rhs);
                cpu.update_flag(Cpsr::C, carry);
                cpu.update_flag(Cpsr::V, (lhs as i32) < (rhs as i32));
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
                ..
            } => {
                let value = Handlers::resolve_operand(src, cpu);
                cpu.write_register(dst, value);
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
}
