use bitmatch::bitmatch;
use std::fmt::Display;

#[derive(PartialEq, Debug)]
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
    pub fn from(value: u32) -> Condition {
        match value {
            0b0000 => Condition::Equal,
            0b0001 => Condition::NotEqual,
            0b0010 => Condition::UnsignedHigherOrSame,
            0b0011 => Condition::UnsignedLower,
            0b0100 => Condition::Negative,
            0b0101 => Condition::PositiveOrZero,
            0b0110 => Condition::Overflow,
            0b0111 => Condition::NoOverflow,
            0b1000 => Condition::UnsignedHigher,
            0b1001 => Condition::UnsignedLowerOrSame,
            0b1010 => Condition::GreaterOrEqual,
            0b1011 => Condition::LessThan,
            0b1100 => Condition::GreaterThan,
            0b1101 => Condition::LessThanOrEqual,
            0b1110 => Condition::Always,
            _ => panic!("Unknown condition code: {}", value),
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

#[derive(PartialEq, Debug)]
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
    Spsr,
    SpsrFlag,
}

impl Register {
    pub fn from(value: u32) -> Register {
        match value {
            0b0000 => Register::R0,
            0b0001 => Register::R1,
            0b0010 => Register::R2,
            0b0011 => Register::R3,
            0b0100 => Register::R4,
            0b0101 => Register::R5,
            0b0110 => Register::R6,
            0b0111 => Register::R7,
            0b1000 => Register::R8,
            0b1001 => Register::R9,
            0b1010 => Register::R10,
            0b1011 => Register::R11,
            0b1100 => Register::R12,
            0b1101 => Register::R13,
            0b1110 => Register::R14,
            0b1111 => Register::R15,
            _ => panic!("Unknown register code: {:b}", value),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum ShiftType {
    LogicalLeft(u32),
    LogicalRight(u32),
    ArithmeticRight(u32),
    RotateRight(u32),
}

impl ShiftType {
    pub fn from(shift_type: u32, value: u32) -> ShiftType {
        match shift_type {
            0b00 => ShiftType::LogicalLeft(value),
            0b01 => ShiftType::LogicalRight(value),
            0b10 => ShiftType::ArithmeticRight(value),
            0b11 => ShiftType::RotateRight(value),
            _ => panic!("Unknown shift type: {}", shift_type),
        }
    }
}

impl Display for ShiftType {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ShiftType::LogicalLeft(value) => write!(f, "lsl #{}", value),
            ShiftType::LogicalRight(value) => write!(f, "lsr #{}", value),
            ShiftType::ArithmeticRight(value) => write!(f, "asr #{}", value),
            ShiftType::RotateRight(value) => write!(f, "ror #{}", value),
        }
    }
}

#[derive(PartialEq, Debug)]
pub enum Operand {
    Immediate(u32, Option<ShiftType>),
    Offset(i32),
    Register(Register, Option<ShiftType>),
    RegisterList(Vec<Register>),
}

#[derive(PartialEq, Debug)]
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
    Svc,
}

impl Opcode {
    pub fn is_test(&self) -> bool {
        *self == Opcode::Cmp || *self == Opcode::Tst || *self == Opcode::Teq || *self == Opcode::Cmn
    }

    pub fn is_load_store(&self) -> bool {
        *self == Opcode::Ldr || *self == Opcode::Str || *self == Opcode::Ldm || *self == Opcode::Stm
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
            Opcode::Svc => write!(f, "svc"),
        }
    }
}

#[derive(PartialEq, Debug)]
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

#[derive(PartialEq, Debug)]
pub enum OffsetOperation {
    Add,
    Sub,
}

#[derive(Debug)]
pub struct Instruction {
    pub opcode: Opcode,
    pub condition: Condition,
    pub set_condition_flags: bool,
    pub operand1: Option<Operand>,
    pub operand2: Option<Operand>,
    pub operand3: Option<Operand>,
    pub transfer_length: Option<TransferLength>,
    pub offset_direction: Option<OffsetOperation>,
}

#[allow(unused_variables)]
impl Instruction {
    pub fn decode(opcode: u32, is_thumb: bool) -> Instruction {
        if is_thumb {
            Instruction::decode_thumb(opcode & 0xffff)
        } else {
            Instruction::decode_armv4t(opcode)
        }
    }

    #[bitmatch]
    fn decode_armv4t(opcode: u32) -> Instruction {
        #[bitmatch]
        match opcode {
            // Supervisor Call (SVC)
            "1110_1111_iiii_iiii_iiii_iiii_iiii_iiii" => Instruction {
                opcode: Opcode::Svc,
                condition: Condition::Always,
                set_condition_flags: false,
                operand1: Some(Operand::Immediate(i, None)),
                operand2: None,
                operand3: None,
                ..Instruction::default()
            },
            // Branch and Exchange (BX)
            "cccc_0001_0010_1111_1111_1111_0001_rrrr" => {
                let condition = Condition::from(c);
                let register = Register::from(r);

                Instruction {
                    opcode: Opcode::Bx,
                    condition,
                    set_condition_flags: false,
                    operand1: Some(Operand::Register(register, None)),
                    operand2: None,
                    operand3: None,
                    ..Instruction::default()
                }
            }
            // Branch (B) and Branch with Link (BL)
            "cccc_101l_oooo_oooo_oooo_oooo_oooo_oooo" => {
                // 101 = Branch, l = has link
                let condition = Condition::from(c);
                let offset = (((o << 2) as i32) << 6) >> 6; // sign extend 24-bit offset

                // branch target is calculated by PC + (offset * 4)
                // this requires PC to be ahead at time of decode to be correct
                // pc should be 2 instructions ahead

                Instruction {
                    opcode: if l == 1 { Opcode::Bl } else { Opcode::B },
                    condition,
                    set_condition_flags: false,
                    operand1: Some(Operand::Offset(offset)),
                    operand2: None,
                    operand3: None,
                    ..Instruction::default()
                }
            }
            // PSR Transfer (MRS)
            "cccc_0001_0s00_1111_dddd_0000_0000_0000" => {
                let condition = Condition::from(c);
                let source = if s == 1 {
                    Register::Spsr
                } else {
                    Register::Cpsr
                };
                let destination = Register::from(d);

                Instruction {
                    opcode: Opcode::Mrs,
                    condition,
                    set_condition_flags: false,
                    operand1: Some(Operand::Register(destination, None)),
                    operand2: Some(Operand::Register(source, None)),
                    operand3: None,
                    ..Instruction::default()
                }
            }
            // PSR Transfer (MSR) for register contents
            "cccc_0001_0d10_1001_1111_0000_0000_ssss" => {
                let condition = Condition::from(c);
                let source = Register::from(s);
                let destination = if d == 1 {
                    Register::Spsr
                } else {
                    Register::Cpsr
                };

                Instruction {
                    opcode: Opcode::Msr,
                    condition,
                    set_condition_flags: false,
                    operand1: Some(Operand::Register(destination, None)),
                    operand2: Some(Operand::Register(source, None)),
                    operand3: None,
                    ..Instruction::default()
                }
            }
            // PSR Transfer (MSR) for register contents or immediate value to PSR flags
            "cccc_00i1_0d10_1000_1111_ssss_ssss_ssss" => {
                let condition = Condition::from(c);
                let destination = if d == 1 {
                    Register::SpsrFlag
                } else {
                    Register::CpsrFlag
                };

                let operand2 = if i == 1 {
                    let imm = s & 0b1111_1111;
                    let rotate = ((s & 0b1111_0000_0000) >> 8) * 2; // 0, 2, 4, 6; increments of 2

                    if rotate == 0 {
                        Operand::Immediate(imm, None)
                    } else {
                        Operand::Immediate(imm, Some(ShiftType::RotateRight(rotate)))
                    }
                } else {
                    let s = s & 0b1111;
                    Operand::Register(Register::from(s), None)
                };

                Instruction {
                    opcode: Opcode::Msr,
                    condition,
                    set_condition_flags: false,
                    operand1: Some(Operand::Register(destination, None)),
                    operand2: Some(operand2),
                    operand3: None,
                    ..Instruction::default()
                }
            }
            // Halfword Data Transfer (LDRH/STRH)
            "cccc_000p_uiwl_yyyy_xxxx_oooo_1sh1_zzzz" => {
                let condition = Condition::from(c);
                let dst = Register::from(x);
                let src = Register::from(y);
                let is_load = l == 1;

                let offset = if i == 1 {
                    // Immediate Operand
                    let z = z & 0b1111;
                    Operand::Immediate(z, None)
                } else {
                    // Register Operand 2
                    let shift_amount = (z & 0b1111_1000_0000) >> 7;
                    let shift_type = (z & 0b0000_0110_0000) >> 5;

                    if shift_amount == 0 {
                        Operand::Register(Register::from(z), None)
                    } else {
                        Operand::Register(
                            Register::from(z),
                            Some(ShiftType::from(shift_type, shift_amount)),
                        )
                    }
                };

                Instruction {
                    opcode: if is_load { Opcode::Ldr } else { Opcode::Str },
                    condition,
                    set_condition_flags: false,
                    operand1: Some(Operand::Register(dst, None)),
                    operand2: Some(Operand::Register(src, None)),
                    operand3: Some(offset),
                    transfer_length: Some(TransferLength::HalfWord),
                    offset_direction: if u == 1 {
                        Some(OffsetOperation::Add)
                    } else {
                        Some(OffsetOperation::Sub)
                    },
                }
            }
            // Data Processing
            "cccc_00io_ooos_yyyy_xxxx_zzzz_zzzz_zzzz" => {
                let condition = Condition::from(c);
                let opcode = Instruction::translate_opcode_armv4t(o);
                let set_condition_flags = s == 1;

                let dst = Operand::Register(Register::from(x), None);

                let rn = if opcode == Opcode::Mvn || opcode == Opcode::Mov {
                    None
                } else {
                    Some(Operand::Register(Register::from(y), None))
                };

                let operand2 = if i == 0 {
                    // Register Operand 2
                    let shift_amount = (z & 0b1111_1000_0000) >> 7;
                    let shift_type = (z & 0b0000_0110_0000) >> 5;
                    let register = z & 0b0001_1111;

                    /*
                       When the second operand is specified to be a shifted register, the operation of the
                       barrel shifter is controlled by the Shift field in the instruction. This field indicates the type
                       of shift to be performed (logical left or right, arithmetic right or rotate right). The amount
                       by which the register should be shifted may be contained in an immediate field in the
                       instruction, or in the bottom byte of another register (other than R15). The encoding for
                       the different shift types is shown in Figure 4-5: ARM shift operations.
                       TODO: lower half of the register
                    */

                    if shift_amount == 0 {
                        Operand::Register(Register::from(register), None)
                    } else {
                        Operand::Register(
                            Register::from(register),
                            Some(ShiftType::from(shift_type, shift_amount)),
                        )
                    }
                } else {
                    // Immediate Operand 2
                    let rotate = ((z & 0b1111_0000_0000) >> 8) * 2; // 0, 2, 4, 6; increments of 2
                    let z = z & 0b1111_1111;

                    if rotate == 0 {
                        Operand::Immediate(z, None)
                    } else {
                        Operand::Immediate(z, Some(ShiftType::RotateRight(rotate)))
                    }
                };

                if opcode.is_test() {
                    // TST, TEQ, CMP, CMN do not have a destination register,
                    // they only set the condition flags

                    return Instruction {
                        opcode,
                        condition,
                        set_condition_flags,
                        operand1: rn,
                        operand2: Some(operand2),
                        operand3: None,
                        ..Instruction::default()
                    };
                }

                if opcode == Opcode::Mov || opcode == Opcode::Mvn {
                    // MOV and MVN do not have a source register
                    return Instruction {
                        opcode,
                        condition,
                        set_condition_flags,
                        operand1: Some(dst),
                        operand2: Some(operand2),
                        operand3: None,
                        ..Instruction::default()
                    };
                } else {
                    return Instruction {
                        opcode,
                        condition,
                        set_condition_flags,
                        operand1: Some(dst),
                        operand2: rn,
                        operand3: Some(operand2),
                        ..Instruction::default()
                    };
                }
            }
            // Block Data Transfer (LDM/STM)
            "cccc_100p_uswl_bbbb_rrrr_rrrr_rrrr_rrrr" => {
                let condition = Condition::from(c);
                let base_register = Register::from(b);

                /*
                    lilyu — Today at 5:18 PM
                    STMDB and LDMIA with SP as the base register have the aliases PUSH and POP respectively
                */

                let (opcode, operand1, operand2) = if l == 0 && base_register == Register::R13 {
                    (
                        Opcode::Push,
                        Some(Operand::RegisterList(Instruction::extract_register_list(r))),
                        None,
                    )
                } else if l == 1 && base_register == Register::R13 {
                    (
                        Opcode::Pop,
                        Some(Operand::RegisterList(Instruction::extract_register_list(r))),
                        None,
                    )
                } else {
                    if l == 0 {
                        (
                            Opcode::Stm,
                            Some(Operand::Register(base_register, None)),
                            Some(Operand::RegisterList(Instruction::extract_register_list(r))),
                        )
                    } else {
                        (
                            Opcode::Ldm,
                            Some(Operand::Register(base_register, None)),
                            Some(Operand::RegisterList(Instruction::extract_register_list(r))),
                        )
                    }
                };

                // TODO: lots of bits missing

                Instruction {
                    opcode,
                    condition,
                    set_condition_flags: false,
                    operand1,
                    operand2,
                    operand3: None,
                    ..Instruction::default()
                }
            }
            // Single Data Transfer (LDR/STR)
            "cccc_01ip_ubwl_yyyy_xxxx_zzzz_zzzz_zzzz" => {
                let condition = Condition::from(c);
                let is_load = l == 1;
                let base_register = Register::from(y);
                let src_or_dst_register = Register::from(x);

                let offset = if i == 0 {
                    // Immediate Operand
                    Operand::Immediate(z & 0b1111_1111_1111, None)
                } else {
                    // Register Operand 2
                    let shift_amount = (z & 0b1111_1000_0000) >> 7;
                    let shift_type = (z & 0b0000_0110_0000) >> 5;
                    let register = z & 0b0001_1111;

                    if shift_amount == 0 {
                        Operand::Register(Register::from(register), None)
                    } else {
                        Operand::Register(
                            Register::from(register),
                            Some(ShiftType::from(shift_type, shift_amount)),
                        )
                    }
                };

                Instruction {
                    opcode: if is_load { Opcode::Ldr } else { Opcode::Str },
                    condition,
                    set_condition_flags: false,
                    operand1: Some(Operand::Register(src_or_dst_register, None)),
                    operand2: Some(Operand::Register(base_register, None)),
                    operand3: Some(offset),
                    transfer_length: if b == 1 {
                        Some(TransferLength::Byte)
                    } else {
                        Some(TransferLength::Word)
                    },
                    offset_direction: if u == 1 {
                        Some(OffsetOperation::Add)
                    } else {
                        Some(OffsetOperation::Sub)
                    },
                }
            }
            _ => panic!("Unknown instruction: {:08x} | {:032b}", opcode, opcode),
        }
    }

    #[bitmatch]
    fn decode_thumb(opcode: u32) -> Instruction {
        #[bitmatch]
        match opcode {
            // move/compare/add/subtract immediate
            "001o_orrr_iiii_iiii" => {
                let opcode = match o {
                    0b00 => Opcode::Mov,
                    0b01 => Opcode::Cmp,
                    0b10 => Opcode::Add,
                    0b11 => Opcode::Sub,
                    _ => unreachable!(),
                };
                let operand1 = Register::from(r);
                let operand2 = Operand::Immediate(i, None);

                Instruction {
                    opcode,
                    condition: Condition::Always,
                    set_condition_flags: false,
                    operand1: Some(Operand::Register(operand1, None)),
                    operand2: Some(operand2),
                    operand3: None,
                    ..Instruction::default()
                }
            }
            // Hi register operations/branch exchange
            "0100_01oo_xyss_sddd" => {
                let (opcode, operand1, operand2) = match (o, x, y) {
                    (0b00, 0, 1) => (
                        Opcode::Add,
                        Some(Operand::Register(Register::from(d), None)),
                        Some(Operand::Register(Register::from(8 + s), None)),
                    ),
                    (0b00, 1, 0) => (
                        Opcode::Add,
                        Some(Operand::Register(Register::from(8 + d), None)),
                        Some(Operand::Register(Register::from(s), None)),
                    ),
                    (0b00, 1, 1) => (
                        Opcode::Add,
                        Some(Operand::Register(Register::from(8 + d), None)),
                        Some(Operand::Register(Register::from(8 + s), None)),
                    ),
                    (0b01, 0, 1) => (
                        Opcode::Cmp,
                        Some(Operand::Register(Register::from(d), None)),
                        Some(Operand::Register(Register::from(8 + s), None)),
                    ),
                    (0b01, 1, 0) => (
                        Opcode::Cmp,
                        Some(Operand::Register(Register::from(8 + d), None)),
                        Some(Operand::Register(Register::from(s), None)),
                    ),
                    (0b01, 1, 1) => (
                        Opcode::Cmp,
                        Some(Operand::Register(Register::from(8 + d), None)),
                        Some(Operand::Register(Register::from(8 + s), None)),
                    ),
                    (0b10, 0, 1) => (
                        Opcode::Mov,
                        Some(Operand::Register(Register::from(d), None)),
                        Some(Operand::Register(Register::from(8 + s), None)),
                    ),
                    (0b10, 1, 0) => (
                        Opcode::Mov,
                        Some(Operand::Register(Register::from(8 + d), None)),
                        Some(Operand::Register(Register::from(s), None)),
                    ),
                    (0b10, 1, 1) => (
                        Opcode::Mov,
                        Some(Operand::Register(Register::from(8 + d), None)),
                        Some(Operand::Register(Register::from(8 + s), None)),
                    ),
                    (0b11, 0, 0) => (
                        Opcode::Bx,
                        Some(Operand::Register(Register::from(s), None)),
                        None,
                    ),
                    (0b11, 0, 1) => (
                        Opcode::Bx,
                        Some(Operand::Register(Register::from(8 + s), None)),
                        None,
                    ),
                    _ => unreachable!(),
                };

                Instruction {
                    opcode,
                    operand1,
                    operand2,
                    ..Instruction::default()
                }
            }
            // load address
            "1010_sddd_cccc_cccc" => {
                let source = match s {
                    0 => Register::R15,
                    1 => Register::R13,
                    _ => unreachable!(),
                };
                let destination = Register::from(d);
                let offset = c << 2;

                Instruction {
                    opcode: Opcode::Add,
                    operand1: Some(Operand::Register(destination, None)),
                    operand2: Some(Operand::Register(source, None)),
                    operand3: Some(Operand::Immediate(offset, None)),
                    ..Instruction::default()
                }
            }
            // Push and Pop
            "1011_l10r_xxxx_xxxx" => Instruction {
                opcode: if l == 0 { Opcode::Push } else { Opcode::Pop },
                condition: Condition::Always,
                set_condition_flags: false,
                operand1: Some(Operand::RegisterList(Instruction::extract_register_list(x))),
                operand2: None,
                operand3: None,
                ..Instruction::default()
            },
            _ => panic!("Unknown instruction: {:04x} | {:016b}", opcode, opcode),
        }
    }

    fn translate_opcode_armv4t(opcode: u32) -> Opcode {
        match opcode {
            0b0000 => Opcode::And,
            0b0001 => Opcode::Eor,
            0b0010 => Opcode::Sub,
            0b0011 => Opcode::Rsb,
            0b0100 => Opcode::Add,
            0b0101 => Opcode::Adc,
            0b0110 => Opcode::Sbc,
            0b0111 => Opcode::Rsc,
            0b1000 => Opcode::Tst,
            0b1001 => Opcode::Teq,
            0b1010 => Opcode::Cmp,
            0b1011 => Opcode::Cmn,
            0b1100 => Opcode::Orr,
            0b1101 => Opcode::Mov,
            0b1110 => Opcode::Bic,
            0b1111 => Opcode::Mvn,
            _ => panic!("Unknown opcode: {:04b}", opcode),
        }
    }

    fn extract_register_list(value: u32) -> Vec<Register> {
        let mut registers = Vec::new();
        for i in 0..16 {
            if value & (1 << i) != 0 {
                registers.push(Register::from(i as u32));
            }
        }
        registers
    }
}

impl Default for Instruction {
    fn default() -> Self {
        Instruction {
            opcode: Opcode::And,
            condition: Condition::Always,
            set_condition_flags: false,
            operand1: None,
            operand2: None,
            operand3: None,
            transfer_length: None,
            offset_direction: None,
        }
    }
}

impl Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        // Note to self: only show condition flags if not a test instruction,
        // they are always set, aka implicite

        write!(
            f,
            "{}{}{}{}",
            self.opcode,
            self.transfer_length
                .as_ref()
                .unwrap_or(&TransferLength::Word),
            self.condition,
            if self.set_condition_flags && !self.opcode.is_test() {
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
                        OffsetOperation::Add => "",
                        OffsetOperation::Sub => "-",
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

        Ok(())
    }
}

impl Display for Operand {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Operand::Immediate(value, option) if option.is_none() => write!(f, "#0x{:02x}", value),
            Operand::Immediate(value, Some(option)) => {
                write!(
                    f,
                    "#0x{:02x}, {} [eval: 0x{:02x}]",
                    value,
                    option,
                    match option {
                        ShiftType::LogicalLeft(shift) => value << shift,
                        ShiftType::LogicalRight(shift) => value >> shift,
                        ShiftType::ArithmeticRight(shift) => ((*value as i32) >> shift) as u32, // TODO: wrong
                        ShiftType::RotateRight(rotate) => value.rotate_right(*rotate),
                    }
                )
            }
            Operand::Register(register, option) if option.is_none() => write!(f, "{}", register),
            Operand::Register(register, Some(option)) => write!(f, "{}, {}", register, option),
            Operand::Offset(value) => write!(f, "#0x{:02x}", value),
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
        }
    }
}
