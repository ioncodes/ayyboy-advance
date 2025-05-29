use bitmatch::bitmatch;
use std::fmt::Display;

#[derive(PartialEq, Debug, Clone)]
pub enum Condition {
    Equal,
    NotEqual,
    UnsignedHigherOrSame,
    UnsignedLower,
    Negative,
    PositiveOrZero,
    Overflow,
    NoOverflow,
    UnsignedHigher,
    UnsignedLowerOrSame,
    GreaterOrEqual,
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    Always,
}

impl Condition {
    pub fn from(value: u32) -> Result<Condition, String> {
        match value {
            0b0000 => Ok(Condition::Equal),
            0b0001 => Ok(Condition::NotEqual),
            0b0010 => Ok(Condition::UnsignedHigherOrSame),
            0b0011 => Ok(Condition::UnsignedLower),
            0b0100 => Ok(Condition::Negative),
            0b0101 => Ok(Condition::PositiveOrZero),
            0b0110 => Ok(Condition::Overflow),
            0b0111 => Ok(Condition::NoOverflow),
            0b1000 => Ok(Condition::UnsignedHigher),
            0b1001 => Ok(Condition::UnsignedLowerOrSame),
            0b1010 => Ok(Condition::GreaterOrEqual),
            0b1011 => Ok(Condition::LessThan),
            0b1100 => Ok(Condition::GreaterThan),
            0b1101 => Ok(Condition::LessThanOrEqual),
            0b1110 => Ok(Condition::Always),
            _ => Err(format!("Unknown condition code: {:b}", value)),
        }
    }
}

impl Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Condition::Equal => write!(f, "eq"),
            Condition::NotEqual => write!(f, "ne"),
            Condition::UnsignedHigherOrSame => write!(f, "cs"),
            Condition::UnsignedLower => write!(f, "cc"),
            Condition::Negative => write!(f, "mi"),
            Condition::PositiveOrZero => write!(f, "pl"),
            Condition::Overflow => write!(f, "vs"),
            Condition::NoOverflow => write!(f, "vc"),
            Condition::UnsignedHigher => write!(f, "hi"),
            Condition::UnsignedLowerOrSame => write!(f, "ls"),
            Condition::GreaterOrEqual => write!(f, "ge"),
            Condition::LessThan => write!(f, "lt"),
            Condition::GreaterThan => write!(f, "gt"),
            Condition::LessThanOrEqual => write!(f, "le"),
            Condition::Always => write!(f, ""),
        }
    }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub enum Register {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    R6,
    R7,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
    Cpsr,
    CpsrFlag,
    CpsrControl,
    CpsrFlagControl,
    Spsr,
    SpsrFlag,
    SpsrControl,
    SpsrFlagControl,
    PsrNone, // Nop, TODO: rewrite PSR flag access
}

impl Register {
    pub fn from(value: u32) -> Result<Register, String> {
        match value {
            0b0000 => Ok(Register::R0),
            0b0001 => Ok(Register::R1),
            0b0010 => Ok(Register::R2),
            0b0011 => Ok(Register::R3),
            0b0100 => Ok(Register::R4),
            0b0101 => Ok(Register::R5),
            0b0110 => Ok(Register::R6),
            0b0111 => Ok(Register::R7),
            0b1000 => Ok(Register::R8),
            0b1001 => Ok(Register::R9),
            0b1010 => Ok(Register::R10),
            0b1011 => Ok(Register::R11),
            0b1100 => Ok(Register::R12),
            0b1101 => Ok(Register::R13),
            0b1110 => Ok(Register::R14),
            0b1111 => Ok(Register::R15),
            _ => Err(format!("Unknown register code: {:b}", value)),
        }
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum ShiftSource {
    Register(Register),
    Immediate(u32),
}

impl Display for ShiftSource {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ShiftSource::Register(register) => write!(f, "{}", register),
            ShiftSource::Immediate(value) => write!(f, "#{}", value),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum ShiftType {
    LogicalLeft(ShiftSource),
    LogicalRight(ShiftSource),
    ArithmeticRight(ShiftSource),
    RotateRight(ShiftSource),
    RotateRightExtended,
}

impl ShiftType {
    pub fn from(shift_type: u32, value: ShiftSource) -> Result<ShiftType, String> {
        // The form of the shift field which might be expected to give
        // ROR #0 is used to encode a special function of the barrel
        // shifter, rotate right extended (RRX). This instruction rotates
        // thx Atem!

        // The form of the shift field which might be expected to correspond
        // to LSR #0 is used to encode LSR #32, which has a
        // zero result with bit 31 of Rm as the carry output.

        // The form of the shift field which might be expected to give
        // ASR #0 is used to encode ASR #32. Bit 31 of Rm is again
        // used as the carry output, and each bit of operand 2 is also
        // equal to bit 31 of Rm.

        match shift_type {
            0b00 => Ok(ShiftType::LogicalLeft(value)),
            0b01 => match value {
                ShiftSource::Immediate(0) => Ok(ShiftType::LogicalRight(ShiftSource::Immediate(32))),
                ShiftSource::Immediate(i) => Ok(ShiftType::LogicalRight(ShiftSource::Immediate(i))),
                _ => Ok(ShiftType::LogicalRight(value)),
            },
            0b10 => match value {
                ShiftSource::Immediate(0) => Ok(ShiftType::ArithmeticRight(ShiftSource::Immediate(32))),
                ShiftSource::Immediate(i) => Ok(ShiftType::ArithmeticRight(ShiftSource::Immediate(i))),
                _ => Ok(ShiftType::ArithmeticRight(value)),
            },
            0b11 => match value {
                ShiftSource::Immediate(0) => Ok(ShiftType::RotateRightExtended),
                ShiftSource::Immediate(i) => Ok(ShiftType::RotateRight(ShiftSource::Immediate(i))),
                ShiftSource::Register(_) => Ok(ShiftType::RotateRight(value)),
            },
            _ => Err(format!("Unknown shift type: {}", shift_type)),
        }
    }
}

impl Display for ShiftType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ShiftType::LogicalLeft(src) => write!(f, "lsl {}", src),
            ShiftType::LogicalRight(src) => write!(f, "lsr {}", src),
            ShiftType::ArithmeticRight(src) => write!(f, "asr {}", src),
            ShiftType::RotateRight(src) => write!(f, "ror {}", src),
            ShiftType::RotateRightExtended => write!(f, "rrx"),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Operand {
    Immediate(u32, Option<ShiftType>),
    Offset(i32),
    Register(Register, Option<ShiftType>),
    RegisterList(Vec<Register>),
}

impl Operand {
    pub fn is_register(&self, register: &Register) -> bool {
        match self {
            Operand::Register(r, _) => r == register,
            _ => false,
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum Opcode {
    B,
    Bl,
    Bx,
    And,
    Eor,
    Sub,
    Rsb,
    Add,
    Adc,
    Sbc,
    Rsc,
    Tst,
    Teq,
    Cmp,
    Cmn,
    Orr,
    Mov,
    Bic,
    Mvn,
    Mrs,
    Msr,
    Ldm,
    Stm,
    Push,
    Pop,
    Ldr,
    Str,
    Swi,
    Lsl,
    Lsr,
    Asr,
    Ror,
    Mul,
    Mla,
    Umull,
    Umlal,
    Smull,
    Smlal,
    Neg,
    Swp,
}

impl Opcode {
    pub fn is_test(&self) -> bool {
        *self == Opcode::Cmp || *self == Opcode::Tst || *self == Opcode::Teq || *self == Opcode::Cmn
    }

    pub fn is_load_store(&self) -> bool {
        *self == Opcode::Ldr || *self == Opcode::Str
    }
}

impl Display for Opcode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Opcode::B => write!(f, "b"),
            Opcode::Bl => write!(f, "bl"),
            Opcode::Bx => write!(f, "bx"),
            Opcode::And => write!(f, "and"),
            Opcode::Eor => write!(f, "eor"),
            Opcode::Sub => write!(f, "sub"),
            Opcode::Rsb => write!(f, "rsb"),
            Opcode::Add => write!(f, "add"),
            Opcode::Adc => write!(f, "adc"),
            Opcode::Sbc => write!(f, "sbc"),
            Opcode::Rsc => write!(f, "rsc"),
            Opcode::Tst => write!(f, "tst"),
            Opcode::Teq => write!(f, "teq"),
            Opcode::Cmp => write!(f, "cmp"),
            Opcode::Cmn => write!(f, "cmn"),
            Opcode::Orr => write!(f, "orr"),
            Opcode::Mov => write!(f, "mov"),
            Opcode::Bic => write!(f, "bic"),
            Opcode::Mvn => write!(f, "mvn"),
            Opcode::Mrs => write!(f, "mrs"),
            Opcode::Msr => write!(f, "msr"),
            Opcode::Ldm => write!(f, "ldm"),
            Opcode::Stm => write!(f, "stm"),
            Opcode::Push => write!(f, "push"),
            Opcode::Pop => write!(f, "pop"),
            Opcode::Ldr => write!(f, "ldr"),
            Opcode::Str => write!(f, "str"),
            Opcode::Swi => write!(f, "swi"),
            Opcode::Lsl => write!(f, "lsl"),
            Opcode::Lsr => write!(f, "lsr"),
            Opcode::Asr => write!(f, "asr"),
            Opcode::Ror => write!(f, "ror"),
            Opcode::Mul => write!(f, "mul"),
            Opcode::Mla => write!(f, "mla"),
            Opcode::Umull => write!(f, "umull"),
            Opcode::Umlal => write!(f, "umlal"),
            Opcode::Smull => write!(f, "smull"),
            Opcode::Smlal => write!(f, "smlal"),
            Opcode::Neg => write!(f, "neg"),
            Opcode::Swp => write!(f, "swp"),
        }
    }
}

#[derive(PartialEq, Debug, Clone)]
pub enum TransferLength {
    Byte,
    HalfWord,
    Word,
}

impl Display for TransferLength {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            TransferLength::Byte => write!(f, "b"),
            TransferLength::HalfWord => write!(f, "h"),
            TransferLength::Word => write!(f, ""), // word is implied
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Direction {
    Up,
    Down,
}

impl Display for Direction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Direction::Up => write!(f, ""),
            Direction::Down => write!(f, "-"),
        }
    }
}

#[derive(PartialEq, Debug, Clone, Copy)]
pub enum Indexing {
    Pre,
    Post,
}

#[derive(Debug, Clone)]
pub struct Instruction {
    pub opcode: Opcode,
    pub condition: Condition,
    pub set_psr_flags: bool,
    pub operand1: Option<Operand>,
    pub operand2: Option<Operand>,
    pub operand3: Option<Operand>,
    pub operand4: Option<Operand>,
    pub transfer_length: Option<TransferLength>,
    pub signed_transfer: bool,
    pub offset_direction: Option<Direction>,
    pub writeback: bool,
    pub indexing: Option<Indexing>,
}

#[allow(unused_variables)]
impl Instruction {
    pub fn decode(opcode: u32, is_thumb: bool) -> Result<Instruction, String> {
        if is_thumb {
            Instruction::decode_thumb(opcode)
        } else {
            Instruction::decode_armv4t(opcode)
        }
    }

    pub fn nop() -> Instruction {
        Instruction {
            opcode: Opcode::Mov,
            operand1: Some(Operand::Register(Register::R0, None)),
            operand2: Some(Operand::Register(Register::R0, None)),
            ..Instruction::default()
        }
    }

    #[bitmatch]
    fn decode_armv4t(opcode: u32) -> Result<Instruction, String> {
        #[bitmatch]
        match opcode {
            // Software Interrupt (SWI) [also known as Supervisor Call (SVC)]
            "1110_1111_iiii_iiii_iiii_iiii_iiii_iiii" => Ok(Instruction {
                opcode: Opcode::Swi,
                condition: Condition::Always,
                set_psr_flags: false,
                operand1: Some(Operand::Immediate(i, None)),
                operand2: None,
                operand3: None,
                ..Instruction::default()
            }),
            // Branch and Exchange (BX)
            "cccc_0001_0010_1111_1111_1111_0001_rrrr" => {
                let condition = Condition::from(c)?;
                let register = Register::from(r)?;

                Ok(Instruction {
                    opcode: Opcode::Bx,
                    condition,
                    set_psr_flags: false,
                    operand1: Some(Operand::Register(register, None)),
                    operand2: None,
                    operand3: None,
                    ..Instruction::default()
                })
            }
            // Branch (B) and Branch with Link (BL)
            "cccc_101l_oooo_oooo_oooo_oooo_oooo_oooo" => {
                // 101 = Branch, l = has link
                let condition = Condition::from(c)?;
                let offset = (((o << 2) as i32) << 6) >> 6; // sign extend 24-bit offset

                // branch target is calculated by PC + (offset * 4)
                // this requires PC to be ahead at time of decode to be correct
                // pc should be 2 instructions ahead

                Ok(Instruction {
                    opcode: if l == 1 { Opcode::Bl } else { Opcode::B },
                    condition,
                    set_psr_flags: false,
                    operand1: Some(Operand::Offset(offset)),
                    operand2: None,
                    operand3: None,
                    ..Instruction::default()
                })
            }
            // Multiply and Multiply-Accumulate (MUL, MLA)
            "cccc_0000_00as_dddd_xxxx_yyyy_1001_zzzz" => {
                let condition = Condition::from(c)?;
                let set_psr_flags = s == 1;
                let accumulate = a == 1;

                let rm = Register::from(z)?;
                let rd = Register::from(d)?;
                let rn = Register::from(x)?;
                let rs = Register::from(y)?;

                Ok(if !accumulate {
                    Instruction {
                        opcode: Opcode::Mul,
                        condition,
                        set_psr_flags,
                        operand1: Some(Operand::Register(rd, None)),
                        operand2: Some(Operand::Register(rm, None)),
                        operand3: Some(Operand::Register(rs, None)),
                        ..Instruction::default()
                    }
                } else {
                    Instruction {
                        opcode: Opcode::Mla,
                        condition,
                        set_psr_flags,
                        operand1: Some(Operand::Register(rd, None)),
                        operand2: Some(Operand::Register(rm, None)),
                        operand3: Some(Operand::Register(rs, None)),
                        operand4: Some(Operand::Register(rn, None)),
                        ..Instruction::default()
                    }
                })
            }
            // Multiply Long and Multiply-Accumulate Long (MULL, MLAL)
            "cccc_0000_1uat_hhhh_llll_ssss_1001_mmmm" => {
                let condition = Condition::from(c)?;
                let set_psr_flags = t == 1;
                let accumulate = a == 1;
                let unsigned = u == 0;

                let rm = Register::from(m)?;
                let rs = Register::from(s)?;
                let rd_hi = Register::from(h)?;
                let rd_lo = Register::from(l)?;

                Ok(Instruction {
                    opcode: match (accumulate, unsigned) {
                        (false, false) => Opcode::Smull,
                        (false, true) => Opcode::Umull,
                        (true, false) => Opcode::Smlal,
                        (true, true) => Opcode::Umlal,
                    },
                    condition,
                    set_psr_flags,
                    operand1: Some(Operand::Register(rd_lo, None)),
                    operand2: Some(Operand::Register(rd_hi, None)),
                    operand3: Some(Operand::Register(rm, None)),
                    operand4: Some(Operand::Register(rs, None)),
                    ..Instruction::default()
                })
            }
            // Single Data Swap (SWP)
            "cccc_0001_0l00_bbbb_dddd_0000_1001_ssss" => {
                let condition = Condition::from(c)?;
                let dst = Register::from(d)?;
                let src = Register::from(s)?;
                let base = Register::from(b)?;

                Ok(Instruction {
                    opcode: Opcode::Swp,
                    condition,
                    operand1: Some(Operand::Register(dst, None)),
                    operand2: Some(Operand::Register(src, None)),
                    operand3: Some(Operand::Register(base, None)),
                    transfer_length: Some(if l == 1 {
                        TransferLength::Byte
                    } else {
                        TransferLength::Word
                    }),
                    ..Instruction::default()
                })
            }
            // Halfword and Signed Data Transfer (LDRH/STRH/LDRSB/LDRSH)
            "cccc_000p_uiwl_yyyy_xxxx_oooo_1sh1_zzzz" => {
                let condition = Condition::from(c)?;
                let dst = Register::from(x)?;
                let src = Register::from(y)?;
                let is_load = l == 1;

                let (operand1, operand2, operand3) = if i == 0 {
                    // Register Offset
                    (
                        Some(Operand::Register(Register::from(x)?, None)),
                        Some(Operand::Register(Register::from(y)?, None)),
                        Some(Operand::Register(Register::from(z)?, None)),
                    )
                } else {
                    // Immediate Offset
                    (
                        Some(Operand::Register(Register::from(x)?, None)),
                        Some(Operand::Register(Register::from(y)?, None)),
                        Some(Operand::Immediate((o << 4) | z, None)),
                    )
                };

                // "In the case of post-indexed addressing, the write back bit is redundant and
                // is always set to zero, since the old base value can be retained if necessary by setting
                // the offset to zero. Therefore post-indexed data transfers always write back the
                // modified base."

                Ok(Instruction {
                    opcode: if is_load { Opcode::Ldr } else { Opcode::Str },
                    condition,
                    set_psr_flags: false,
                    operand1,
                    operand2,
                    operand3,
                    transfer_length: match (s, h) {
                        (0, 1) => Some(TransferLength::HalfWord), // unsigned
                        (1, 0) => Some(TransferLength::Byte),     // signed
                        (1, 1) => Some(TransferLength::HalfWord), // signed
                        _ => return Err("Invalid transfer length for LDRH/STRH".to_string()),
                    },
                    signed_transfer: s == 1,
                    offset_direction: if u == 1 {
                        Some(Direction::Up)
                    } else {
                        Some(Direction::Down)
                    },
                    indexing: if p == 1 {
                        Some(Indexing::Pre)
                    } else {
                        Some(Indexing::Post)
                    },
                    writeback: w == 1 || p == 0,
                    ..Instruction::default()
                })
            }
            // Data Processing
            "cccc_00io_ooos_yyyy_xxxx_zzzz_zzzz_zzzz" => {
                let condition = Condition::from(c)?;
                let decoded_opcode = Instruction::translate_opcode_armv4t(o)?;
                let set_psr_flags = s == 1;

                if !set_psr_flags && decoded_opcode.is_test() {
                    #[bitmatch]
                    match opcode {
                        // PSR Transfer (MRS)
                        "cccc_0001_0s00_1111_dddd_0000_0000_0000" => {
                            let condition = Condition::from(c)?;
                            let source = if s == 1 { Register::Spsr } else { Register::Cpsr };
                            let destination = Register::from(d)?;

                            return Ok(Instruction {
                                opcode: Opcode::Mrs,
                                condition,
                                set_psr_flags: false,
                                operand1: Some(Operand::Register(destination, None)),
                                operand2: Some(Operand::Register(source, None)),
                                operand3: None,
                                ..Instruction::default()
                            });
                        }
                        // // PSR Transfer (MSR) for register contents
                        // TODO: can we remove this safely?
                        // "cccc_0001_0d10_1001_1111_0000_0000_ssss" => {
                        //     let condition = Condition::from(c)?;
                        //     let source = Register::from(s)?;
                        //     let destination = if d == 1 { Register::Spsr } else { Register::Cpsr };

                        //     return Ok(Instruction {
                        //         opcode: Opcode::Msr,
                        //         condition,
                        //         set_psr_flags: false,
                        //         operand1: Some(Operand::Register(destination, None)),
                        //         operand2: Some(Operand::Register(source, None)),
                        //         operand3: None,
                        //         ..Instruction::default()
                        //     });
                        // }
                        // PSR Transfer (MSR) for register contents or immediate value to PSR flags
                        "cccc_00i1_0d10_f??x_1111_ssss_ssss_ssss" => {
                            // https://problemkaputt.de/gbatek-arm-opcodes-psr-transfer-mrs-msr.htm
                            let condition = Condition::from(c)?;
                            let destination = match (d, f, x) {
                                (1, 1, 0) => Register::SpsrFlag,
                                (1, 0, 1) => Register::SpsrControl,
                                (0, 1, 0) => Register::CpsrFlag,
                                (0, 0, 1) => Register::CpsrControl,
                                (1, 1, 1) => Register::SpsrFlagControl,
                                (0, 1, 1) => Register::CpsrFlagControl,
                                (0, 0, 0) => Register::PsrNone,
                                _ => Err(format!("Invalid PSR transfer destination: d={}, f={}, x={}", d, f, x))?,
                            };

                            let operand2 = if i == 1 {
                                let imm = s & 0b1111_1111;
                                let rotate = ((s & 0b1111_0000_0000) >> 8) * 2; // 0, 2, 4, 6; increments of 2

                                if rotate == 0 {
                                    Operand::Immediate(imm, None)
                                } else {
                                    Operand::Immediate(
                                        imm,
                                        Some(ShiftType::RotateRight(ShiftSource::Immediate(rotate))),
                                    )
                                }
                            } else {
                                let s = s & 0b1111;
                                Operand::Register(Register::from(s)?, None)
                            };

                            return Ok(Instruction {
                                opcode: Opcode::Msr,
                                condition,
                                set_psr_flags: false,
                                operand1: Some(Operand::Register(destination, None)),
                                operand2: Some(operand2),
                                operand3: None,
                                ..Instruction::default()
                            });
                        }
                        _ => {}
                    }
                }

                let dst = Operand::Register(Register::from(x)?, None);

                let rn = if decoded_opcode == Opcode::Mvn || decoded_opcode == Opcode::Mov {
                    None
                } else {
                    Some(Operand::Register(Register::from(y)?, None))
                };

                let operand2 = if i == 0 {
                    // Register Operand 2

                    #[bitmatch]
                    match z {
                        "rrrr_0tt1_dddd" => Operand::Register(
                            Register::from(d)?,
                            Some(ShiftType::from(t, ShiftSource::Register(Register::from(r)?))?),
                        ),
                        "ssss_stt0_dddd" => {
                            Operand::Register(Register::from(d)?, Some(ShiftType::from(t, ShiftSource::Immediate(s))?))
                        }
                        _ => unreachable!(),
                    }
                } else {
                    // Immediate Operand 2
                    let rotate = ((z & 0b1111_0000_0000) >> 8) * 2; // 0, 2, 4, 6; increments of 2
                    let z = z & 0b1111_1111;

                    if rotate == 0 {
                        Operand::Immediate(z, None)
                    } else {
                        Operand::Immediate(z, Some(ShiftType::RotateRight(ShiftSource::Immediate(rotate))))
                    }
                };

                if decoded_opcode.is_test() {
                    // TST, TEQ, CMP, CMN do not have a destination register,
                    // they only set the condition flags

                    return Ok(Instruction {
                        opcode: decoded_opcode,
                        condition,
                        set_psr_flags: true,
                        operand1: rn,
                        operand2: Some(operand2),
                        operand3: None,
                        ..Instruction::default()
                    });
                }

                if decoded_opcode == Opcode::Mov || decoded_opcode == Opcode::Mvn {
                    // MOV and MVN do not have a source register
                    return Ok(Instruction {
                        opcode: decoded_opcode,
                        condition,
                        set_psr_flags,
                        operand1: Some(dst),
                        operand2: Some(operand2),
                        operand3: None,
                        ..Instruction::default()
                    });
                } else {
                    return Ok(Instruction {
                        opcode: decoded_opcode,
                        condition,
                        set_psr_flags,
                        operand1: Some(dst),
                        operand2: rn,
                        operand3: Some(operand2),
                        ..Instruction::default()
                    });
                }
            }
            // Block Data Transfer (LDM/STM)
            "cccc_100p_uswl_bbbb_rrrr_rrrr_rrrr_rrrr" => {
                let condition = Condition::from(c)?;
                let base_register = Register::from(b)?;

                Ok(Instruction {
                    opcode: if l == 1 { Opcode::Ldm } else { Opcode::Stm },
                    condition,
                    set_psr_flags: s == 1,
                    operand1: Some(Operand::Register(base_register, None)),
                    operand2: Some(Operand::RegisterList(Instruction::extract_register_list(r)?)),
                    indexing: if p == 1 {
                        Some(Indexing::Pre)
                    } else {
                        Some(Indexing::Post)
                    },
                    writeback: w == 1,
                    offset_direction: if u == 1 {
                        Some(Direction::Up)
                    } else {
                        Some(Direction::Down)
                    },
                    ..Instruction::default()
                })
            }
            // Single Data Transfer (LDR/STR)
            "cccc_01ip_ubwl_yyyy_xxxx_zzzz_zzzz_zzzz" => {
                let condition = Condition::from(c)?;
                let is_load = l == 1;
                let base_register = Register::from(y)?;
                let src_or_dst_register = Register::from(x)?;

                let offset = if i == 0 {
                    // Immediate Operand
                    Operand::Immediate(z & 0b1111_1111_1111, None)
                } else {
                    // Register Operand 2
                    let shift_amount = (z & 0b1111_1000_0000) >> 7;
                    let shift_type = (z & 0b0000_0110_0000) >> 5;
                    let register = z & 0b0001_1111;

                    Operand::Register(
                        Register::from(register)?,
                        Some(ShiftType::from(shift_type, ShiftSource::Immediate(shift_amount))?),
                    )
                };

                // "In the case of post-indexed addressing, the write back bit is redundant and
                // is always set to zero, since the old base value can be retained if necessary by setting
                // the offset to zero. Therefore post-indexed data transfers always write back the
                // modified base."

                Ok(Instruction {
                    opcode: if is_load { Opcode::Ldr } else { Opcode::Str },
                    condition,
                    set_psr_flags: false,
                    operand1: Some(Operand::Register(src_or_dst_register, None)),
                    operand2: Some(Operand::Register(base_register, None)),
                    operand3: Some(offset),
                    transfer_length: if b == 1 {
                        Some(TransferLength::Byte)
                    } else {
                        Some(TransferLength::Word)
                    },
                    offset_direction: if u == 1 {
                        Some(Direction::Up)
                    } else {
                        Some(Direction::Down)
                    },
                    indexing: if p == 1 {
                        Some(Indexing::Pre)
                    } else {
                        Some(Indexing::Post)
                    },
                    writeback: w == 1 || p == 0,
                    ..Instruction::default()
                })
            }
            _ => Err(format!("Unknown instruction: {:08x} | {:032b}", opcode, opcode)),
        }
    }

    #[bitmatch]
    fn decode_thumb(opcode: u32) -> Result<Instruction, String> {
        #[bitmatch]
        match opcode & 0xffff {
            // add/subtract
            "0001_1ico_ooss_sddd" => {
                let opcode = if c == 0 { Opcode::Add } else { Opcode::Sub };
                let operand1 = Register::from(d)?;
                let operand2 = Register::from(s)?;
                let operand3 = if i == 0 {
                    Operand::Register(Register::from(o)?, None)
                } else {
                    Operand::Immediate(o, None)
                };

                Ok(Instruction {
                    opcode,
                    condition: Condition::Always,
                    set_psr_flags: true,
                    operand1: Some(Operand::Register(operand1, None)),
                    operand2: Some(Operand::Register(operand2, None)),
                    operand3: Some(operand3),
                    ..Instruction::default()
                })
            }
            // Move shifted register
            "000c_cooo_ooss_sddd" => {
                let opcode = match c {
                    0b00 => Opcode::Lsl,
                    0b01 => Opcode::Lsr,
                    0b10 => Opcode::Asr,
                    _ => return Err("Invalid shift type for move shifted register".to_string()),
                };
                let operand1 = Register::from(d)?;
                let operand2 = Register::from(s)?;

                Ok(Instruction {
                    opcode,
                    condition: Condition::Always,
                    set_psr_flags: true,
                    operand1: Some(Operand::Register(operand1, None)),
                    operand2: Some(Operand::Register(operand2, None)),
                    operand3: Some(Operand::Immediate(o, None)),
                    ..Instruction::default()
                })
            }
            // Move/compare/add/subtract immediate
            "001o_orrr_iiii_iiii" => {
                let opcode = match o {
                    0b00 => Opcode::Mov,
                    0b01 => Opcode::Cmp,
                    0b10 => Opcode::Add,
                    0b11 => Opcode::Sub,
                    _ => Err("Invalid opcode for move/compare/add/subtract immediate")?,
                };
                let operand1 = Register::from(r)?;
                let operand2 = Operand::Immediate(i, None);

                Ok(Instruction {
                    opcode,
                    condition: Condition::Always,
                    set_psr_flags: true,
                    operand1: Some(Operand::Register(operand1, None)),
                    operand2: Some(operand2),
                    operand3: None,
                    ..Instruction::default()
                })
            }
            // load/store with immediate offset
            "011w_looo_oobb_bddd" => {
                let opcode = if l == 1 { Opcode::Ldr } else { Opcode::Str };
                let operand1 = Register::from(d)?;
                let operand2 = Register::from(b)?;

                // For word accesses (B = 0), the value specified by
                // #Imm is a full 7-bit address, but must be word-aligned (ie
                // with bits 1:0 set to 0), since the assembler places #Imm >>
                // 2 in the Offset5 field.
                let operand3 = if w == 0 {
                    Operand::Immediate(o << 2, None)
                } else {
                    Operand::Immediate(o, None)
                };

                Ok(Instruction {
                    opcode,
                    condition: Condition::Always,
                    set_psr_flags: false,
                    operand1: Some(Operand::Register(operand1, None)),
                    operand2: Some(Operand::Register(operand2, None)),
                    operand3: Some(operand3),
                    transfer_length: Some(if w == 1 {
                        TransferLength::Byte
                    } else {
                        TransferLength::Word
                    }),
                    offset_direction: Some(Direction::Up),
                    indexing: Some(Indexing::Pre),
                    ..Instruction::default()
                })
            }
            // ALU operations
            "0100_00oo_ooss_sddd" => {
                let opcode = Instruction::translate_opcode_thumb(o)?;
                let operand1 = Register::from(d)?;
                let operand2 = Register::from(s)?;

                Ok(Instruction {
                    opcode,
                    condition: Condition::Always,
                    set_psr_flags: true,
                    operand1: Some(Operand::Register(operand1, None)),
                    operand2: Some(Operand::Register(operand2, None)),
                    ..Instruction::default()
                })
            }
            // Hi register operations/branch exchange
            "0100_01oo_xyss_sddd" => {
                let (opcode, operand1, operand2) = match (o, x, y) {
                    (0b00, 0, 1) => (
                        Opcode::Add,
                        Some(Operand::Register(Register::from(d)?, None)),
                        Some(Operand::Register(Register::from(8 + s)?, None)),
                    ),
                    (0b00, 1, 0) => (
                        Opcode::Add,
                        Some(Operand::Register(Register::from(8 + d)?, None)),
                        Some(Operand::Register(Register::from(s)?, None)),
                    ),
                    (0b00, 1, 1) => (
                        Opcode::Add,
                        Some(Operand::Register(Register::from(8 + d)?, None)),
                        Some(Operand::Register(Register::from(8 + s)?, None)),
                    ),
                    (0b01, 0, 1) => (
                        Opcode::Cmp,
                        Some(Operand::Register(Register::from(d)?, None)),
                        Some(Operand::Register(Register::from(8 + s)?, None)),
                    ),
                    (0b01, 1, 0) => (
                        Opcode::Cmp,
                        Some(Operand::Register(Register::from(8 + d)?, None)),
                        Some(Operand::Register(Register::from(s)?, None)),
                    ),
                    (0b01, 1, 1) => (
                        Opcode::Cmp,
                        Some(Operand::Register(Register::from(8 + d)?, None)),
                        Some(Operand::Register(Register::from(8 + s)?, None)),
                    ),
                    (0b10, 0, 1) => (
                        Opcode::Mov,
                        Some(Operand::Register(Register::from(d)?, None)),
                        Some(Operand::Register(Register::from(8 + s)?, None)),
                    ),
                    (0b10, 1, 0) => (
                        Opcode::Mov,
                        Some(Operand::Register(Register::from(8 + d)?, None)),
                        Some(Operand::Register(Register::from(s)?, None)),
                    ),
                    (0b10, 1, 1) => (
                        Opcode::Mov,
                        Some(Operand::Register(Register::from(8 + d)?, None)),
                        Some(Operand::Register(Register::from(8 + s)?, None)),
                    ),
                    (0b11, 0, 0) => (Opcode::Bx, Some(Operand::Register(Register::from(s)?, None)), None),
                    (0b11, 0, 1) => (Opcode::Bx, Some(Operand::Register(Register::from(8 + s)?, None)), None),
                    _ => Err("Invalid opcode for Hi register operations")?,
                };

                // Note: In this group only CMP (Op = 01) sets the CPSR
                // condition codes.
                let set_psr_flags = opcode == Opcode::Cmp;

                Ok(Instruction {
                    opcode,
                    operand1,
                    operand2,
                    set_psr_flags,
                    ..Instruction::default()
                })
            }
            // PC-relative load
            "0100_1ddd_iiii_iiii" => {
                let destination = Register::from(d)?;
                let offset = i << 2;

                Ok(Instruction {
                    opcode: Opcode::Ldr,
                    condition: Condition::Always,
                    set_psr_flags: false,
                    operand1: Some(Operand::Register(destination, None)),
                    operand2: Some(Operand::Register(Register::R15, None)),
                    operand3: Some(Operand::Immediate(offset, None)),
                    offset_direction: Some(Direction::Up),
                    indexing: Some(Indexing::Pre),
                    transfer_length: Some(TransferLength::Word),
                    ..Instruction::default()
                })
            }
            // load/store with register offset
            "0101_lw0o_oobb_bddd" => {
                let opcode = if l == 1 { Opcode::Ldr } else { Opcode::Str };
                let destination = Register::from(d)?;
                let base = Register::from(b)?;
                let offset = Register::from(o)?;

                Ok(Instruction {
                    opcode,
                    condition: Condition::Always,
                    set_psr_flags: false,
                    operand1: Some(Operand::Register(destination, None)),
                    operand2: Some(Operand::Register(base, None)),
                    operand3: Some(Operand::Register(offset, None)),
                    transfer_length: if w == 1 {
                        Some(TransferLength::Byte)
                    } else {
                        Some(TransferLength::Word)
                    },
                    offset_direction: Some(Direction::Up),
                    indexing: Some(Indexing::Pre),
                    ..Instruction::default()
                })
            }
            // load/store sign-extended byte/halfword
            "0101_hs1o_oobb_bddd" => {
                let opcode = match (s, h) {
                    (0, 0) => Opcode::Str,
                    _ => Opcode::Ldr,
                };
                let destination = Register::from(d)?;
                let base = Register::from(b)?;
                let offset = Register::from(o)?;

                Ok(Instruction {
                    opcode,
                    condition: Condition::Always,
                    set_psr_flags: false,
                    operand1: Some(Operand::Register(destination, None)),
                    operand2: Some(Operand::Register(base, None)),
                    operand3: Some(Operand::Register(offset, None)),
                    transfer_length: match (s, h) {
                        (0, 0) => Some(TransferLength::HalfWord),
                        (0, 1) => Some(TransferLength::HalfWord),
                        (1, 0) => Some(TransferLength::Byte),
                        (1, 1) => Some(TransferLength::HalfWord),
                        _ => Err("Invalid transfer length for load/store sign-extended byte/halfword")?,
                    },
                    offset_direction: Some(Direction::Up),
                    indexing: Some(Indexing::Pre),
                    signed_transfer: s == 1,
                    ..Instruction::default()
                })
            }
            // load/store halfword
            "1000_looo_oobb_bddd" => {
                let opcode = if l == 1 { Opcode::Ldr } else { Opcode::Str };
                let destination = Register::from(d)?;
                let base = Register::from(b)?;
                let offset = Operand::Immediate(o << 1, None);

                Ok(Instruction {
                    opcode,
                    condition: Condition::Always,
                    set_psr_flags: false,
                    operand1: Some(Operand::Register(destination, None)),
                    operand2: Some(Operand::Register(base, None)),
                    operand3: Some(offset),
                    transfer_length: Some(TransferLength::HalfWord),
                    offset_direction: Some(Direction::Up),
                    indexing: Some(Indexing::Pre),
                    ..Instruction::default()
                })
            }
            // SP-relative load/store
            "1001_lddd_iiii_iiii" => {
                let opcode = if l == 1 { Opcode::Ldr } else { Opcode::Str };
                let destination = Register::from(d)?;
                let offset = i << 2;

                Ok(Instruction {
                    opcode,
                    condition: Condition::Always,
                    set_psr_flags: false,
                    operand1: Some(Operand::Register(destination, None)),
                    operand2: Some(Operand::Register(Register::R13, None)),
                    operand3: Some(Operand::Immediate(offset, None)),
                    offset_direction: Some(Direction::Up),
                    indexing: Some(Indexing::Pre),
                    transfer_length: Some(TransferLength::Word),
                    ..Instruction::default()
                })
            }
            // Load address
            "1010_sddd_cccc_cccc" => {
                let source = match s {
                    0 => Register::R15,
                    1 => Register::R13,
                    _ => unreachable!(),
                };
                let destination = Register::from(d)?;
                let offset = c << 2;

                Ok(Instruction {
                    opcode: Opcode::Add,
                    operand1: Some(Operand::Register(destination, None)),
                    operand2: Some(Operand::Register(source, None)),
                    operand3: Some(Operand::Immediate(offset, None)),
                    ..Instruction::default()
                })
            }
            // add offset to Stack Pointer
            "1011_0000_sooo_oooo" => {
                let offset = if s == 1 { -((o << 2) as i32) } else { (o << 2) as i32 };

                Ok(Instruction {
                    opcode: Opcode::Add,
                    operand1: Some(Operand::Register(Register::R13, None)),
                    operand2: Some(Operand::Offset(offset)),
                    ..Instruction::default()
                })
            }
            // Push and Pop
            "1011_l10r_xxxx_xxxx" => {
                let mut registers = Instruction::extract_register_list(x)?;
                let opcode = match (l, r) {
                    (0, 0) => Opcode::Push,
                    (0, 1) => {
                        registers.push(Register::R14);
                        Opcode::Push
                    }
                    (1, 0) => Opcode::Pop,
                    (1, 1) => {
                        registers.push(Register::R15);
                        Opcode::Pop
                    }
                    _ => Err("Invalid opcode for Push/Pop")?,
                };

                Ok(Instruction {
                    opcode,
                    condition: Condition::Always,
                    set_psr_flags: false,
                    operand1: Some(Operand::RegisterList(registers)),
                    operand2: None,
                    operand3: None,
                    ..Instruction::default()
                })
            }
            // multiple load/store
            "1100_lbbb_rrrr_rrrr" => Ok(Instruction {
                opcode: if l == 0 { Opcode::Stm } else { Opcode::Ldm },
                condition: Condition::Always,
                set_psr_flags: false,
                operand1: Some(Operand::Register(Register::from(b)?, None)),
                operand2: Some(Operand::RegisterList(Instruction::extract_register_list(r)?)),
                operand3: None,
                indexing: Some(Indexing::Post),
                offset_direction: Some(Direction::Up),
                writeback: true,
                ..Instruction::default()
            }),
            // software interrupt
            "1101_1111_iiii_iiii" => Ok(Instruction {
                opcode: Opcode::Swi,
                condition: Condition::Always,
                set_psr_flags: false,
                operand1: Some(Operand::Immediate(i, None)),
                ..Instruction::default()
            }),
            // Conditional Branch
            "1101_cccc_iiii_iiii" => {
                let offset = i as u8;
                let signed_offset = ((i as i8) as i16) << 1;

                Ok(Instruction {
                    opcode: Opcode::B,
                    condition: Condition::from(c)?,
                    set_psr_flags: false,
                    operand1: Some(Operand::Offset(signed_offset as i32)),
                    ..Instruction::default()
                })
            }
            // Unconditional Branch
            "1110_0iii_iiii_iiii" => {
                let signed_offset = (((i as i16) << 5) >> 5) << 1;

                Ok(Instruction {
                    opcode: Opcode::B,
                    condition: Condition::Always,
                    set_psr_flags: false,
                    operand1: Some(Operand::Offset(signed_offset as i32)),
                    ..Instruction::default()
                })
            }
            // Long branch with link
            "1111_hiii_iiii_iiii" => {
                // TODO: Atem — 12:01 AM
                // treating thumb bl as one big 32-bit instr isn't exactly ideal either
                // golden sun for instance just uses one half of it in some cases

                let hi_half = (opcode & 0xFFFF) as u16; // first fetched
                let lo_half = (opcode >> 16) as u16; // second fetched

                // upper 11 bits and lower 11 bits
                let imm_hi = (hi_half & 0x07FF) as i32; // bits 10-0
                let imm_lo = (lo_half & 0x07FF) as i32;

                // build 23-bit signed offset
                let mut offset = (imm_hi << 12) | (imm_lo << 1); // bit0 is always 0

                // sign-extend from bit 22
                offset = (offset << 9) >> 9; // keep 23 bits signed

                Ok(Instruction {
                    opcode: Opcode::Bl,
                    condition: Condition::Always,
                    set_psr_flags: false,
                    operand1: Some(Operand::Offset(offset)),
                    ..Instruction::default()
                })
            }
            _ => Err(format!(
                "Unknown instruction: {:04x} | {:016b}",
                opcode & 0xffff,
                opcode & 0xffff
            )),
        }
    }

    fn translate_opcode_armv4t(opcode: u32) -> Result<Opcode, String> {
        match opcode {
            0b0000 => Ok(Opcode::And),
            0b0001 => Ok(Opcode::Eor),
            0b0010 => Ok(Opcode::Sub),
            0b0011 => Ok(Opcode::Rsb),
            0b0100 => Ok(Opcode::Add),
            0b0101 => Ok(Opcode::Adc),
            0b0110 => Ok(Opcode::Sbc),
            0b0111 => Ok(Opcode::Rsc),
            0b1000 => Ok(Opcode::Tst),
            0b1001 => Ok(Opcode::Teq),
            0b1010 => Ok(Opcode::Cmp),
            0b1011 => Ok(Opcode::Cmn),
            0b1100 => Ok(Opcode::Orr),
            0b1101 => Ok(Opcode::Mov),
            0b1110 => Ok(Opcode::Bic),
            0b1111 => Ok(Opcode::Mvn),
            _ => Err(format!("Unknown opcode: {:04b}", opcode)),
        }
    }

    fn translate_opcode_thumb(opcode: u32) -> Result<Opcode, String> {
        match opcode {
            0b0000 => Ok(Opcode::And),
            0b0001 => Ok(Opcode::Eor),
            0b0010 => Ok(Opcode::Lsl),
            0b0011 => Ok(Opcode::Lsr),
            0b0100 => Ok(Opcode::Asr),
            0b0101 => Ok(Opcode::Adc),
            0b0110 => Ok(Opcode::Sbc),
            0b0111 => Ok(Opcode::Ror),
            0b1000 => Ok(Opcode::Tst),
            0b1001 => Ok(Opcode::Neg),
            0b1010 => Ok(Opcode::Cmp),
            0b1011 => Ok(Opcode::Cmn),
            0b1100 => Ok(Opcode::Orr),
            0b1101 => Ok(Opcode::Mul),
            0b1110 => Ok(Opcode::Bic),
            0b1111 => Ok(Opcode::Mvn),
            _ => Err(format!("Unknown opcode: {:04b}", opcode)),
        }
    }

    fn extract_register_list(value: u32) -> Result<Vec<Register>, String> {
        let mut registers = Vec::new();
        for i in 0..16 {
            if value & (1 << i) != 0 {
                registers.push(Register::from(i as u32)?);
            }
        }
        Ok(registers)
    }
}

impl Default for Instruction {
    fn default() -> Self {
        Instruction {
            opcode: Opcode::And,
            condition: Condition::Always,
            set_psr_flags: false,
            operand1: None,
            operand2: None,
            operand3: None,
            operand4: None,
            transfer_length: None,
            signed_transfer: false,
            offset_direction: None,
            writeback: false,
            indexing: None,
        }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Note to self: only show condition flags if not a test instruction,
        // they are always set, aka implicite

        match self.opcode {
            Opcode::Ldr | Opcode::Str if self.indexing == Some(Indexing::Post) => {
                write!(
                    f,
                    "{}{}{}{}{} {}",
                    self.opcode,
                    self.signed_transfer.then(|| "s").unwrap_or(""),
                    self.transfer_length.as_ref().unwrap_or(&TransferLength::Word),
                    self.condition,
                    if self.set_psr_flags && !self.opcode.is_test() {
                        ".s"
                    } else {
                        ""
                    },
                    self.operand1.as_ref().unwrap(),
                )?;

                if self.writeback {
                    write!(
                        f,
                        ", [{}], {}{}",
                        self.operand2.as_ref().unwrap(),
                        self.offset_direction.as_ref().unwrap(),
                        self.operand3.as_ref().unwrap()
                    )?;
                } else {
                    write!(f, ", [{}]", self.operand2.as_ref().unwrap())?;
                }
            }
            Opcode::Ldr | Opcode::Str if self.indexing == Some(Indexing::Pre) => {
                write!(
                    f,
                    "{}{}{}{}{} {}",
                    self.opcode,
                    self.signed_transfer.then(|| "s").unwrap_or(""),
                    self.transfer_length.as_ref().unwrap_or(&TransferLength::Word),
                    self.condition,
                    if self.set_psr_flags && !self.opcode.is_test() {
                        ".s"
                    } else {
                        ""
                    },
                    self.operand1.as_ref().unwrap(),
                )?;

                write!(
                    f,
                    ", [{}, {}{}]",
                    self.operand2.as_ref().unwrap(),
                    self.offset_direction.as_ref().unwrap(),
                    self.operand3.as_ref().unwrap()
                )?;

                if self.writeback {
                    write!(f, "!")?;
                }
            }
            Opcode::Ldm | Opcode::Stm => {
                let opcode_suffix = match (&self.indexing, &self.offset_direction) {
                    (Some(Indexing::Pre), Some(Direction::Up)) => "ib",
                    (Some(Indexing::Pre), Some(Direction::Down)) => "db",
                    (Some(Indexing::Post), Some(Direction::Up)) => "ia", // technically not required as it's default
                    (Some(Indexing::Post), Some(Direction::Down)) => "da",
                    _ => unreachable!(),
                };
                let opcode = match self.opcode {
                    Opcode::Ldm => format!("ldm{}", opcode_suffix),
                    Opcode::Stm => format!("stm{}", opcode_suffix),
                    _ => unreachable!(),
                };

                write!(
                    f,
                    "{}{} {}{}, {}{}",
                    opcode,
                    self.condition,
                    self.operand1.as_ref().unwrap(),
                    if self.writeback { "!" } else { "" },
                    self.operand2.as_ref().unwrap(),
                    if self.set_psr_flags && !self.opcode.is_test() {
                        "^"
                    } else {
                        ""
                    },
                )?;
            }
            Opcode::Swp => {
                write!(
                    f,
                    "{}{}{}{} {}, {}",
                    self.opcode,
                    self.condition,
                    match self.transfer_length {
                        Some(TransferLength::Byte) => "b",
                        Some(TransferLength::Word) => "",
                        _ => unreachable!(),
                    },
                    if self.set_psr_flags && !self.opcode.is_test() {
                        ".s"
                    } else {
                        ""
                    },
                    self.operand1.as_ref().unwrap(),
                    self.operand2.as_ref().unwrap(),
                )?;
                write!(f, ", [{}]", self.operand3.as_ref().unwrap())?;
            }
            // Opcode::Add | Opcode::Sub
            //     if let Some(Operand::Register(reg, None)) = &self.operand2
            //         && *reg == Register::R15 =>
            // {
            //     write!(
            //         f,
            //         "adr{}{} {}, [{}, {}]",
            //         self.condition,
            //         if self.set_psr_flags && !self.opcode.is_test() {
            //             ".s"
            //         } else {
            //             ""
            //         },
            //         self.operand1.as_ref().unwrap(),
            //         self.operand2.as_ref().unwrap(),
            //         self.operand3.as_ref().unwrap(),
            //     )?;
            // }
            _ => {
                write!(
                    f,
                    "{}{}{}{}",
                    self.opcode,
                    self.transfer_length.as_ref().unwrap_or(&TransferLength::Word),
                    self.condition,
                    if self.set_psr_flags && !self.opcode.is_test() {
                        ".s"
                    } else {
                        ""
                    }
                )?;
                if let Some(operand) = &self.operand1 {
                    write!(f, " {}", operand)?;
                }

                if let Some(operand) = &self.operand2 {
                    write!(f, ", ")?;

                    if self.opcode.is_load_store() {
                        write!(f, "[")?;
                    }

                    write!(f, "{}", operand)?;
                }

                if let Some(operand) = &self.operand3 {
                    write!(
                        f,
                        ", {}{}",
                        if let Some(op) = &self.offset_direction {
                            match op {
                                Direction::Up => "",
                                Direction::Down => "-",
                            }
                        } else {
                            ""
                        },
                        operand
                    )?;
                }

                if self.opcode.is_load_store() {
                    write!(f, "]")?;
                }

                if let Some(operand) = &self.operand4 {
                    write!(f, ", {}", operand)?;
                }
            }
        }

        Ok(())
    }
}

impl Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Operand::Immediate(value, option) if option.is_none() => write!(f, "0x{:04x}", value),
            Operand::Immediate(value, Some(option)) => {
                write!(f, "0x{:04x}, {}", value, option)
            }
            Operand::Register(register, option) if option.is_none() => write!(f, "{}", register),
            Operand::Register(register, Some(option)) => write!(f, "{}, {}", register, option),
            Operand::Offset(value) if *value > 0 => write!(f, "+0x{:04x}", value),
            Operand::Offset(value) if *value < 0 => write!(f, "-0x{:04x}", -1 * value),
            Operand::Offset(value) => write!(f, "0x{:04x}", value),
            Operand::RegisterList(registers) => {
                let output = registers
                    .iter()
                    .map(|r| format!("{}", r))
                    .collect::<Vec<String>>()
                    .join(", ");
                write!(f, "{{{}}}", output)
            }
            _ => panic!("Unknown operand type"),
        }
    }
}

impl Display for Register {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Register::R0 => write!(f, "r0"),
            Register::R1 => write!(f, "r1"),
            Register::R2 => write!(f, "r2"),
            Register::R3 => write!(f, "r3"),
            Register::R4 => write!(f, "r4"),
            Register::R5 => write!(f, "r5"),
            Register::R6 => write!(f, "r6"),
            Register::R7 => write!(f, "r7"),
            Register::R8 => write!(f, "r8"),
            Register::R9 => write!(f, "r9"),
            Register::R10 => write!(f, "r10"),
            Register::R11 => write!(f, "r11"),
            Register::R12 => write!(f, "r12"),
            Register::R13 => write!(f, "sp"),
            Register::R14 => write!(f, "lr"),
            Register::R15 => write!(f, "pc"),
            Register::Cpsr => write!(f, "cpsr"),
            Register::Spsr => write!(f, "spsr"),
            Register::CpsrFlag => write!(f, "cpsr_flg"),
            Register::SpsrFlag => write!(f, "spsr_flg"),
            Register::CpsrControl => write!(f, "cpsr_ctl"),
            Register::SpsrControl => write!(f, "spsr_ctl"),
            Register::CpsrFlagControl => write!(f, "cpsr_fc"),
            Register::SpsrFlagControl => write!(f, "spsr_fc"),
            Register::PsrNone => write!(f, "psr_none"),
        }
    }
}
