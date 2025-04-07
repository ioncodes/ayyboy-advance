use super::cpu::Cpu;
use super::decoder::{Condition, Instruction, Opcode, Operand, ShiftSource, ShiftType};
use super::registers::Psr;
use crate::arm7tdmi::decoder::{Direction, Indexing, Register, TransferLength};
use crate::arm7tdmi::mode::ProcessorMode;
use crate::memory::mmio::Mmio;
use spdlog::prelude::*;

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
                cpu.registers.r[15] = 0x08;

                // copy the current cpsr to spsr[current_mode]
                cpu.write_register(&Register::Spsr, cpu.read_register(&Register::Cpsr));

                // set the current mode to supervisor
                cpu.set_processor_mode(ProcessorMode::Supervisor);

                // set the link register to the address of the instruction after the SWI
                let addr_next_instr = pc - if cpu.is_thumb() { 2 } else { 4 };
                cpu.write_register(&Register::R14, addr_next_instr);

                // switch to ARM state
                cpu.registers.cpsr.set(Psr::T, false);
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
                let result = match instr.opcode {
                    Opcode::Cmp => {
                        let (result, carry) = x.overflowing_sub(y);
                        cpu.update_flag(Psr::C, !carry); // Invert carry for CMP (borrow flag)
                        cpu.update_flag(Psr::V, ((x ^ y) & (x ^ result) & 0x8000_0000) != 0);
                        result
                    }
                    Opcode::Cmn => {
                        let (result, carry) = x.overflowing_add(y);
                        cpu.update_flag(Psr::C, carry); // Carry as is for CMN (unsigned overflow)
                        cpu.update_flag(Psr::V, ((x ^ result) & (y ^ result) & 0x8000_0000) != 0);
                        result
                    }
                    _ => unreachable!(),
                };

                cpu.update_flag(Psr::N, (result as i32) < 0);
                cpu.update_flag(Psr::Z, result == 0);

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
                set_psr_flags,
                ..
            } => {
                let value = Handlers::resolve_operand(src, cpu, *set_psr_flags);
                let extra_fetch = if src.is_register(&Register::R15)
                    && let Some(ShiftSource::Register(_)) = Handlers::try_fetch_shifted_operand(src)
                {
                    4
                } else {
                    0
                };
                let result = value + extra_fetch;
                cpu.write_register(dst, result);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                }
            }
            Instruction {
                opcode: Opcode::Mvn,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(src),
                set_psr_flags,
                ..
            } => {
                let value = Handlers::resolve_operand(src, cpu, *set_psr_flags);
                let extra_fetch = if src.is_register(&Register::R15)
                    && let Some(ShiftSource::Register(_)) = Handlers::try_fetch_shifted_operand(src)
                {
                    4
                } else {
                    0
                };
                let result = !(value + extra_fetch);
                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                signed_transfer,
                offset_direction: Some(operation),
                set_psr_flags,
                indexing: Some(indexing),
                writeback,
                ..
            } => {
                let mut address = cpu.read_register(src);
                let step = Handlers::resolve_operand(step, cpu, *set_psr_flags);

                if *indexing == Indexing::Pre {
                    if *operation == Direction::Up {
                        address = address.wrapping_add(step)
                    } else {
                        address = address.wrapping_sub(step)
                    }
                }

                // align address, https://problemkaputt.de/gbatek.htm#armcpumemoryalignments
                let (mut aligned_address, rotation) = if address % 4 != 0 {
                    let mask = match length {
                        TransferLength::Byte => 0b00,
                        TransferLength::HalfWord if *signed_transfer => 0b11, // ldrsh misaligned reads the byte at the misaligned address
                        TransferLength::HalfWord => 0b01,
                        TransferLength::Word => 0b11,
                    } as u32;
                    let aligned_address = address & !mask;
                    let rotation = (address & mask) * 8;
                    (aligned_address, rotation)
                } else {
                    (address, 0)
                };

                match length {
                    TransferLength::Byte => {
                        let value = if *signed_transfer {
                            // The LDRSB instruction loads the selected Byte into bits 7
                            // to 0 of the destination register and bits 31 to 8 of the desti-
                            // nation register are set to the value of bit 7, the sign bit.
                            let value = mmio.read(aligned_address).rotate_right(rotation);
                            value as i8 as u32
                        } else {
                            mmio.read(aligned_address).rotate_right(rotation) as u32
                        };

                        cpu.write_register(dst, value as u32);

                        if *set_psr_flags {
                            cpu.update_flag(Psr::N, value & 0x80 != 0);
                            cpu.update_flag(Psr::Z, value == 0);
                        }
                    }
                    TransferLength::HalfWord => {
                        let value = if aligned_address == address && *signed_transfer {
                            // The LDRSH instruction loads the selected Half-word into
                            // bits 15 to 0 of the destination register and bits 31 to 16 of
                            // the destination register are set to the value of bit 15, the
                            // sign bit.
                            let value = mmio.read_u16(aligned_address) as u32;
                            let value = value.rotate_right(rotation);
                            let sign_bit = value & (1 << 15);
                            if sign_bit != 0 {
                                value | 0xffff_0000
                            } else {
                                value & 0x0000_ffff
                            }
                        } else if aligned_address != address && *signed_transfer {
                            // Mis-aligned LDRH,LDRSH (does or does not do strange things)
                            // On ARM7 aka ARMv4 aka NDS7/GBA:
                            //   LDRH Rd,[odd]   -->  LDRH Rd,[odd-1] ROR 8  ;read to bit0-7 and bit24-31
                            //   LDRSH Rd,[odd]  -->  LDRSB Rd,[odd]         ;sign-expand BYTE value
                            let value = mmio.read(address); // Bits 0-7
                            value as i8 as u32
                        } else {
                            let value = mmio.read_u16(aligned_address) as u32;
                            value.rotate_right(rotation)
                        };

                        cpu.write_register(dst, value);

                        if *set_psr_flags {
                            cpu.update_flag(Psr::N, value & 0x8000 != 0);
                            cpu.update_flag(Psr::Z, value == 0);
                        }
                    }
                    TransferLength::Word => {
                        let value = mmio.read_u32(aligned_address).rotate_right(rotation);
                        cpu.write_register(dst, value);

                        if *set_psr_flags {
                            cpu.update_flag(Psr::N, value & 0x8000_0000 != 0);
                            cpu.update_flag(Psr::Z, value == 0);
                        }
                    }
                }

                // if dst == src, then the loaded value would overwrite the register after writeback

                if *indexing == Indexing::Post && *dst != *src {
                    if *operation == Direction::Up {
                        aligned_address = aligned_address.wrapping_add(step);
                    } else {
                        aligned_address = aligned_address.wrapping_sub(step);
                    }
                }

                if *writeback && *dst != *src {
                    cpu.write_register(src, aligned_address);
                }
            }
            Instruction {
                opcode: Opcode::Str,
                operand1: Some(Operand::Register(src, None)),
                operand2: Some(Operand::Register(dst, None)),
                operand3: Some(step),
                transfer_length: Some(length),
                offset_direction: Some(operation),
                set_psr_flags,
                indexing: Some(indexing),
                writeback,
                ..
            } => {
                let mut address = cpu.read_register(dst);
                let step = Handlers::resolve_operand(step, cpu, *set_psr_flags);

                if *indexing == Indexing::Pre {
                    if *operation == Direction::Up {
                        address = address.wrapping_add(step)
                    } else {
                        address = address.wrapping_sub(step)
                    }
                }

                if address % 2 != 0 {
                    // align address, https://problemkaputt.de/gbatek.htm#armcpumemoryalignments
                    address &= !((match length {
                        TransferLength::Byte => 0b00,
                        TransferLength::HalfWord => 0b01,
                        TransferLength::Word => 0b11,
                    }) as u32);
                }

                let cpu_read_reg = |reg: &Register| {
                    if *reg == Register::R15 {
                        cpu.read_register(reg) + 4
                    } else {
                        cpu.read_register(reg)
                    }
                };

                match length {
                    TransferLength::Byte => {
                        let value = cpu_read_reg(src) as u8;
                        mmio.write(address, value);
                        if *set_psr_flags {
                            cpu.update_flag(Psr::N, value & 0x80 != 0);
                            cpu.update_flag(Psr::Z, value == 0);
                        }
                    }
                    TransferLength::HalfWord => {
                        let value = cpu_read_reg(src) as u16;
                        mmio.write_u16(address, value);
                        if *set_psr_flags {
                            cpu.update_flag(Psr::N, value & 0x8000 != 0);
                            cpu.update_flag(Psr::Z, value == 0);
                        }
                    }
                    TransferLength::Word => {
                        let value = cpu_read_reg(src);
                        mmio.write_u32(address, value);
                        if *set_psr_flags {
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
                opcode: Opcode::Swp,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: Some(Operand::Register(addr, None)),
                transfer_length: Some(length),
                ..
            } => {
                let addr = cpu.read_register(addr);
                let (aligned_addr, rotation) = if addr % 2 != 0 {
                    let aligned_addr = addr
                        & !((match length {
                            TransferLength::Byte => 0b00,
                            TransferLength::Word => 0b11,
                            _ => unreachable!(),
                        }) as u32);
                    let rotation = (addr - aligned_addr) * 8;
                    (aligned_addr, rotation)
                } else {
                    (addr, 0)
                };

                let original_value = match length {
                    TransferLength::Byte => mmio.read(aligned_addr) as u32,
                    TransferLength::Word => mmio.read_u32(aligned_addr),
                    _ => unreachable!(),
                }
                .rotate_right(rotation);

                match length {
                    TransferLength::Byte => {
                        let value = cpu.read_register(src) as u8;
                        mmio.write(aligned_addr, value);
                    }
                    TransferLength::Word => {
                        let value = cpu.read_register(src);
                        mmio.write_u32(aligned_addr, value);
                    }
                    _ => unreachable!(),
                }
                cpu.write_register(dst, original_value);
            }
            Instruction {
                opcode: Opcode::Ldm,
                operand1: Some(Operand::Register(src_base, None)),
                operand2: Some(Operand::RegisterList(registers)),
                offset_direction: Some(operation),
                indexing: Some(indexing),
                writeback,
                set_psr_flags,
                ..
            } => {
                let cpu_write_register = |cpu: &mut Cpu, register: &Register, value: u32| {
                    if *set_psr_flags {
                        cpu.write_register_for_mode(register, value, ProcessorMode::User);
                    } else {
                        cpu.write_register(register, value);
                    }
                };

                let mut address = cpu.read_register(src_base);

                // Empty Rlist: R15 loaded/stored (ARMv4 only), and Rb=Rb+/-40h (ARMv4-v5).
                // http://problemkaputt.de/gbatek-arm-opcodes-memory-block-data-transfer-ldm-stm.htm
                let increment_amount = if registers.is_empty() { 0x40 } else { 4 };
                let registers = if registers.is_empty() {
                    &vec![Register::R15]
                } else {
                    registers
                };

                for register in registers.iter() {
                    if *indexing == Indexing::Pre {
                        if *operation == Direction::Up {
                            address += increment_amount;
                        } else {
                            address -= increment_amount;
                        }
                    }

                    let value = mmio.read_u32(address & !0b11);
                    cpu_write_register(cpu, register, value);

                    if *indexing == Indexing::Post {
                        if *operation == Direction::Up {
                            address += increment_amount;
                        } else {
                            address -= increment_amount;
                        }
                    }
                }

                // Writeback with Rb included in Rlist: Store OLD base if Rb is FIRST entry
                // in Rlist, otherwise store NEW base (STM/ARMv4), always store OLD base
                // (STM/ARMv5), no writeback (LDM/ARMv4), writeback if Rb is "the ONLY register,
                // or NOT the LAST register" in Rlist (LDM/ARMv5).
                if *writeback
                    && let Some(reg) = registers.first()
                    && reg != src_base
                    && let Some(reg) = registers.last()
                    && reg != src_base
                {
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
                set_psr_flags,
                ..
            } => {
                let mut address = cpu.read_register(dst_base);
                let end_address = registers.iter().fold(address, |addr, _| match *operation {
                    Direction::Up => addr + 4,
                    Direction::Down => addr - 4,
                });

                let cpu_read_register = |register: &Register| {
                    // if the dest base register is in the list,
                    // we don't actually store the value of the register
                    // but rather the value it would have been after the writeback
                    // this should only be respected if the writeback flag is set
                    // and the base is not the first or last register in the list
                    if *writeback
                        && dst_base == register
                        && let Some(first) = registers.first()
                        && first != register
                        && let Some(last) = registers.last()
                        && last != register
                    {
                        return end_address;
                    }

                    if *set_psr_flags {
                        cpu.read_register_for_mode(register, ProcessorMode::User)
                    } else {
                        if *register == Register::R15 {
                            cpu.read_register(register) + 4
                        } else {
                            cpu.read_register(register)
                        }
                    }
                };

                // Empty Rlist: R15 loaded/stored (ARMv4 only), and Rb=Rb+/-40h (ARMv4-v5).
                // http://problemkaputt.de/gbatek-arm-opcodes-memory-block-data-transfer-ldm-stm.htm
                let increment_amount = if registers.is_empty() { 0x40 } else { 4 };
                let registers = if registers.is_empty() {
                    &vec![Register::R15]
                } else {
                    registers
                };

                for register in registers {
                    if *indexing == Indexing::Pre {
                        if *operation == Direction::Up {
                            address += increment_amount;
                        } else {
                            address -= increment_amount;
                        }
                    }

                    if registers.first() != Some(&Register::R15) {
                        let value = cpu_read_register(register);
                        mmio.write_u32(address & !0b11, value);
                    } else {
                        // TODO: what kinda monstrosity is this. rewrite all of ldm/stm
                        // real processor supposedly starts always at the lowest address
                        // and everything is an increment

                        let temp_addr = match (indexing, operation) {
                            (Indexing::Pre, Direction::Down) => address,
                            (Indexing::Pre, Direction::Up) => address - 0x3c,
                            (Indexing::Post, Direction::Up) => address,
                            (Indexing::Post, Direction::Down) => address - 0x3c,
                        };

                        let value = cpu_read_register(register);
                        mmio.write_u32(temp_addr & !0b11, value);
                    }

                    if *indexing == Indexing::Post {
                        if *operation == Direction::Up {
                            address += increment_amount;
                        } else {
                            address -= increment_amount;
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
                set_psr_flags,
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
                let x = Handlers::resolve_operand(x, cpu, *set_psr_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_psr_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };

                let (result, carry) = x.overflowing_add(y);
                let (_, overflow) = (x as i32).overflowing_add(y as i32);
                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                operand2: Some(Operand::Offset(offset)),
                operand3: None,
                set_psr_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let result = x.wrapping_add_signed(*offset);
                cpu.write_register(dst, result);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                }
            }
            Instruction {
                opcode: Opcode::Add,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(src),
                operand3: None,
                set_psr_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = Handlers::resolve_operand(src, cpu, *set_psr_flags);
                let (result, carry) = x.overflowing_add(y);
                let (_, overflow) = (x as i32).overflowing_add(y as i32);
                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
                ..
            } => {
                let carry = cpu.registers.cpsr.contains(Psr::C) as u32; // Grab carry first, as it may be modified due to shifter
                let x = Handlers::resolve_operand(x, cpu, *set_psr_flags);
                let y = Handlers::resolve_operand(y, cpu, *set_psr_flags);

                let (result, carry1) = x.overflowing_add(y);
                let (result, carry2) = result.overflowing_add(carry);

                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
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
                let x = Handlers::resolve_operand(x, cpu, *set_psr_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_psr_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };
                let (result, borrow) = x.overflowing_sub(y);
                let (_, overflow) = (x as i32).overflowing_sub(y as i32);
                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = Handlers::resolve_operand(src, cpu, *set_psr_flags);
                let (result, borrow) = x.overflowing_sub(y);
                let (_, overflow) = (x as i32).overflowing_sub(y as i32);
                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
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
                let x = Handlers::resolve_operand(x, cpu, *set_psr_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_psr_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };
                let carry = cpu.registers.cpsr.contains(Psr::C) as u32;

                let (result, borrow1) = x.overflowing_sub(y);
                let (result, borrow2) = result.overflowing_sub(1 - carry);

                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = Handlers::resolve_operand(src, cpu, *set_psr_flags);
                let carry = cpu.registers.cpsr.contains(Psr::C) as u32;

                let (result, borrow1) = x.overflowing_sub(y);
                let (result, borrow2) = result.overflowing_sub(1 - carry);

                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
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
                let x = Handlers::resolve_operand(x, cpu, *set_psr_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_psr_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };
                let result = x & y;
                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = cpu.read_register(src);
                let result = x & y;
                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
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

                let x = Handlers::resolve_operand(x, cpu, *set_psr_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_psr_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };

                let result = x | y;
                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = cpu.read_register(src);
                let result = x | y;
                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
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
                let x = Handlers::resolve_operand(x, cpu, *set_psr_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_psr_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };
                let result = x ^ y;
                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = cpu.read_register(src);
                let result = x ^ y;
                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
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
                let x = Handlers::resolve_operand(x, cpu, *set_psr_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_psr_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };
                let (result, borrow) = y.overflowing_sub(x);
                let (_, overflow) = (y as i32).overflowing_sub(x as i32);
                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
                ..
            } => {
                // Grab carry first, as it may be modified due to shifter
                let carry = cpu.registers.cpsr.contains(Psr::C) as u32;

                // Extra fetch quirk stuff
                let extra_fetch = match (
                    Handlers::try_fetch_shifted_operand(x),
                    Handlers::try_fetch_shifted_operand(y),
                ) {
                    (_, Some(ShiftSource::Register(_))) => 4,
                    (Some(ShiftSource::Register(_)), _) => 4,
                    _ => 0,
                };

                let x = Handlers::resolve_operand(x, cpu, *set_psr_flags)
                    + if x.is_register(&Register::R15) { extra_fetch } else { 0 };
                let y = Handlers::resolve_operand(y, cpu, *set_psr_flags)
                    + if y.is_register(&Register::R15) { extra_fetch } else { 0 };

                let (result, borrow1) = y.overflowing_sub(x);
                let (result, borrow2) = result.overflowing_sub(1 - carry);
                cpu.write_register(dst, result);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, !borrow1 && !borrow2);

                    cpu.update_flag(Psr::V, false); // TODO:?

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Neg,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: None,
                set_psr_flags,
                ..
            } => {
                let value = cpu.read_register(src);
                let (result, borrow) = 0u32.overflowing_sub(value);
                let (_, overflow) = (0i32).overflowing_sub(value as i32);
                cpu.write_register(dst, result);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                    cpu.update_flag(Psr::C, !borrow);
                    cpu.update_flag(Psr::V, overflow);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Bic,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: None,
                set_psr_flags,
                ..
            } => {
                let result = cpu.read_register(dst) & !cpu.read_register(src);
                cpu.write_register(dst, result);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Bic,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: Some(pos),
                set_psr_flags,
                ..
            } => {
                let pos = Handlers::resolve_operand(pos, cpu, *set_psr_flags);
                let src = cpu.read_register(src);
                let result = src & !pos;
                cpu.write_register(dst, result);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Lsl | Opcode::Lsr | Opcode::Asr,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3,
                set_psr_flags,
                ..
            } => {
                let (value, shift) = match operand3 {
                    Some(Operand::Immediate(shift, _)) => (cpu.read_register(src), *shift),
                    None => {
                        // if operand3 doesn't exist, it's dst := dst << src
                        (cpu.read_register(dst), cpu.read_register(src))
                    }
                    _ => unreachable!(),
                };
                let result = match instr.opcode {
                    Opcode::Lsl => Self::process_shift(
                        value,
                        &ShiftType::LogicalLeft(ShiftSource::Immediate(shift)),
                        cpu,
                        *set_psr_flags,
                    ),
                    Opcode::Lsr => Self::process_shift(
                        value,
                        &ShiftType::LogicalRight(ShiftSource::Immediate(shift)),
                        cpu,
                        *set_psr_flags,
                    ),
                    Opcode::Asr => Self::process_shift(
                        value,
                        &ShiftType::ArithmeticRight(ShiftSource::Immediate(shift)),
                        cpu,
                        *set_psr_flags,
                    ),
                    _ => unreachable!(),
                };
                cpu.write_register(dst, result);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    match instr.opcode {
                        Opcode::Lsl => {
                            cpu.update_flag(Psr::C, value & (1 << (32 - shift)) != 0);
                        }
                        Opcode::Lsr | Opcode::Asr => {
                            cpu.update_flag(Psr::C, value & (1 << (shift - 1)) != 0);
                        }
                        _ => unreachable!(),
                    }

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Ror,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: None,
                set_psr_flags,
                ..
            } => {
                let value = cpu.read_register(dst);
                let rotate = cpu.read_register(src);
                let result = Self::process_shift(
                    value,
                    &ShiftType::RotateRight(ShiftSource::Immediate(rotate)),
                    cpu,
                    *set_psr_flags,
                );
                cpu.write_register(dst, result);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Mul,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(lhs, None)),
                operand3: Some(Operand::Register(rhs, None)),
                operand4: None,
                set_psr_flags,
                ..
            } => {
                let lhs = cpu.read_register(lhs);
                let rhs = cpu.read_register(rhs);
                let result = lhs.wrapping_mul(rhs);
                cpu.write_register(dst, result);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);

                    copy_spsr_to_cpsr_if_necessary(cpu, dst);
                }
            }
            Instruction {
                opcode: Opcode::Mul,
                operand1: Some(Operand::Register(dst, None)),
                operand2: Some(Operand::Register(src, None)),
                operand3: None,
                operand4: None,
                set_psr_flags,
                ..
            } => {
                let x = cpu.read_register(dst);
                let y = cpu.read_register(src);
                let result = x.wrapping_mul(y);
                cpu.write_register(dst, result);

                if *set_psr_flags {
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
                set_psr_flags,
                ..
            } => {
                let lhs = cpu.read_register(lhs);
                let rhs = cpu.read_register(rhs);
                let acc = cpu.read_register(acc);
                let result = lhs.wrapping_mul(rhs).wrapping_add(acc);
                cpu.write_register(dst, result);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                }
            }
            Instruction {
                opcode: Opcode::Umull,
                operand1: Some(Operand::Register(lo, None)),
                operand2: Some(Operand::Register(hi, None)),
                operand3: Some(Operand::Register(lhs, None)),
                operand4: Some(Operand::Register(rhs, None)),
                set_psr_flags,
                ..
            } => {
                let lhs = cpu.read_register(lhs);
                let rhs = cpu.read_register(rhs);
                let result = (lhs as u64).wrapping_mul(rhs as u64);
                cpu.write_register(lo, result as u32);
                cpu.write_register(hi, (result >> 32) as u32);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000_0000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                }
            }
            Instruction {
                opcode: Opcode::Umlal,
                operand1: Some(Operand::Register(lo, None)),
                operand2: Some(Operand::Register(hi, None)),
                operand3: Some(Operand::Register(lhs, None)),
                operand4: Some(Operand::Register(rhs, None)),
                set_psr_flags,
                ..
            } => {
                let lhs = cpu.read_register(lhs);
                let rhs = cpu.read_register(rhs);
                let acc = (cpu.read_register(lo) as u64) | ((cpu.read_register(hi) as u64) << 32);
                let result = acc.wrapping_add((lhs as u64).wrapping_mul(rhs as u64));
                cpu.write_register(lo, result as u32);
                cpu.write_register(hi, (result >> 32) as u32);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, result & 0x8000_0000_0000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                }
            }
            Instruction {
                opcode: Opcode::Smull,
                operand1: Some(Operand::Register(lo, None)),
                operand2: Some(Operand::Register(hi, None)),
                operand3: Some(Operand::Register(lhs, None)),
                operand4: Some(Operand::Register(rhs, None)),
                set_psr_flags,
                ..
            } => {
                let lhs = cpu.read_register(lhs) as i32;
                let rhs = cpu.read_register(rhs) as i32;
                let result = (lhs as i64).wrapping_mul(rhs as i64);
                cpu.write_register(lo, result as u32);
                cpu.write_register(hi, (result >> 32) as u32);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, (result as u64) & 0x8000_0000_0000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                }
            }
            Instruction {
                opcode: Opcode::Smlal,
                operand1: Some(Operand::Register(lo, None)),
                operand2: Some(Operand::Register(hi, None)),
                operand3: Some(Operand::Register(lhs, None)),
                operand4: Some(Operand::Register(rhs, None)),
                set_psr_flags,
                ..
            } => {
                let lhs = cpu.read_register(lhs) as i32;
                let rhs = cpu.read_register(rhs) as i32;
                let acc = (cpu.read_register(lo) as i64) | ((cpu.read_register(hi) as i64) << 32);
                let result = acc.wrapping_add((lhs as i64).wrapping_mul(rhs as i64));
                cpu.write_register(lo, result as u32);
                cpu.write_register(hi, (result >> 32) as u32);

                if *set_psr_flags {
                    cpu.update_flag(Psr::N, (result as u64) & 0x8000_0000_0000_0000 != 0);
                    cpu.update_flag(Psr::Z, result == 0);
                }
            }
            _ => todo!("{:?}", instr),
        }
    }

    fn resolve_operand(operand: &Operand, cpu: &mut Cpu, set_psr_flags: bool) -> u32 {
        match operand {
            Operand::Immediate(value, Some(shift)) => Handlers::process_shift(*value, shift, cpu, set_psr_flags),
            Operand::Immediate(value, None) => *value,
            Operand::Register(register, Some(shift)) => {
                Handlers::process_shift(cpu.read_register(register), shift, cpu, set_psr_flags)
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

    fn process_shift(value: u32, shift: &ShiftType, cpu: &mut Cpu, set_psr_flags: bool) -> u32 {
        match shift {
            ShiftType::LogicalLeft(src) => {
                let shift = Handlers::unwrap_shift_source(cpu, src);

                // Special case for LSL #0 - no shift, carry unchanged
                if shift == 0 {
                    return value;
                }

                // Shift by more than 32 produces 0
                let result = if shift >= 32 { 0 } else { value << shift };

                if set_psr_flags {
                    if shift == 32 {
                        // For shift of 32, carry is bit 0
                        cpu.update_flag(Psr::C, value & 1 != 0);
                    } else if shift > 32 {
                        // For shift > 32, carry is 0
                        cpu.update_flag(Psr::C, false);
                    } else if shift > 0 {
                        // Normal case: carry is the last bit shifted out
                        let mask = 1 << (32 - shift);
                        cpu.update_flag(Psr::C, value & mask != 0);
                    }
                }

                result
            }
            ShiftType::LogicalRight(src) => {
                let shift = Handlers::unwrap_shift_source(cpu, src);

                // LSR #0 is interpreted as LSR #32
                let (result, carry) = if shift == 0 || shift == 32 {
                    // Special case: LSR #0/LSR #32 -> all zeros, carry = bit 31
                    (0, (value & 0x80000000) != 0)
                } else if shift > 32 {
                    // Shift > 32 = all zeros, carry = 0
                    (0, false)
                } else {
                    // Normal case
                    (value >> shift, (value & (1 << (shift - 1))) != 0)
                };

                if set_psr_flags {
                    cpu.update_flag(Psr::C, carry);
                }

                result
            }
            ShiftType::ArithmeticRight(src) => {
                let shift = Handlers::unwrap_shift_source(cpu, src);
                let is_negative = (value & 0x80000000) != 0;

                // ASR #0 is interpreted as ASR #32
                if shift == 0 || shift >= 32 {
                    // Fill with sign bit for shifts of 0 or >= 32
                    let result = if is_negative { 0xffffffff } else { 0 };

                    if set_psr_flags {
                        // Carry out is bit 31 (sign bit)
                        cpu.update_flag(Psr::C, is_negative);
                    }

                    return result;
                }

                // Normal arithmetic shift right (1-31)
                let result = if is_negative {
                    // Need to sign-extend by filling upper bits with 1s
                    (value >> shift) | (0xffffffff << (32 - shift))
                } else {
                    value >> shift
                };

                if set_psr_flags {
                    // Carry is the last bit shifted out
                    cpu.update_flag(Psr::C, (value & (1 << (shift - 1))) != 0);
                }

                result
            }
            ShiftType::RotateRight(src) => {
                let shift = Handlers::unwrap_shift_source(cpu, src);

                // For rotates, shift > 32 is taken modulo 32
                let effective_shift = shift & 0x1f;
                let result = value.rotate_right(effective_shift);

                if set_psr_flags {
                    if effective_shift == 0 {
                        // For ROR #0 (which is interpreted as ROR #32),
                        // carry out is bit 31 (the last bit rotated)
                        cpu.update_flag(Psr::C, (value & 0x80000000) != 0);
                    } else {
                        // For ROR #N (1-31), carry is the last bit rotated out
                        cpu.update_flag(Psr::C, (value & (1 << (effective_shift - 1))) != 0);
                    }
                }

                result
            }
            ShiftType::RotateRightExtended => {
                let new_carry = (value & 1) != 0;
                let result = (value >> 1) | ((cpu.registers.cpsr.contains(Psr::C) as u32) << 31);

                if set_psr_flags {
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
