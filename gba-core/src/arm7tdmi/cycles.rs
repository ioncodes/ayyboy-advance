#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryAccessType {
    Sequential,    // S-cycle: sequential memory access
    NonSequential, // N-cycle: non-sequential memory access
    Internal,      // I-cycle: internal CPU cycle (no memory access)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CycleInfo {
    pub cycles: u32,
    pub access_type: MemoryAccessType,
}

impl CycleInfo {
    pub fn new(cycles: u32, access_type: MemoryAccessType) -> Self {
        Self { cycles, access_type }
    }

    pub fn sequential(cycles: u32) -> Self {
        Self::new(cycles, MemoryAccessType::Sequential)
    }

    pub fn non_sequential(cycles: u32) -> Self {
        Self::new(cycles, MemoryAccessType::NonSequential)
    }

    pub fn internal(cycles: u32) -> Self {
        Self::new(cycles, MemoryAccessType::Internal)
    }
}

/// Memory region cycle costs for different access types
/// Based on GBATEK documentation
pub struct MemoryTiming;

impl MemoryTiming {
    /// Get memory access timing for a given address
    pub fn get_access_cycles(addr: u32, access_type: MemoryAccessType, width: u32) -> u32 {
        match addr {
            // BIOS (0x00000000-0x00003FFF) - Always 1 cycle
            0x00000000..=0x00003FFF => 1,

            // External Work RAM (0x02000000-0x0203FFFF) - 3S/3N cycles
            0x02000000..=0x0203FFFF => match access_type {
                MemoryAccessType::Sequential => 3,
                MemoryAccessType::NonSequential => 3,
                MemoryAccessType::Internal => 1,
            },

            // Internal Work RAM (0x03000000-0x03007FFF) - Always 1 cycle
            0x03000000..=0x03007FFF => 1,

            // I/O Registers (0x04000000-0x040003FE) - Always 1 cycle
            0x04000000..=0x040003FE => 1,

            // Palette RAM (0x05000000-0x050003FF) - Always 1 cycle
            0x05000000..=0x050003FF => 1,

            // VRAM (0x06000000-0x06017FFF) - Always 1 cycle
            0x06000000..=0x06017FFF => 1,

            // OAM (0x07000000-0x070003FF) - Always 1 cycle
            0x07000000..=0x070003FF => 1,

            // Game ROM (0x08000000-0x09FFFFFF, 0x0A000000-0x0BFFFFFF, 0x0C000000-0x0DFFFFFF)
            0x08000000..=0x09FFFFFF | 0x0A000000..=0x0BFFFFFF | 0x0C000000..=0x0DFFFFFF => {
                // Wait state timing - simplified for now
                match access_type {
                    MemoryAccessType::Sequential => {
                        if width == 32 { 2 } else { 1 } // 16-bit: 1S, 32-bit: 2S
                    }
                    MemoryAccessType::NonSequential => {
                        if width == 32 { 5 } else { 4 } // 16-bit: 4N, 32-bit: 5N (assumes WS0)
                    }
                    MemoryAccessType::Internal => 1,
                }
            }

            // Game Save (0x0E000000-0x0E00FFFF) - Variable timing
            0x0E000000..=0x0E00FFFF => match access_type {
                MemoryAccessType::Sequential => 1,
                MemoryAccessType::NonSequential => 1,
                MemoryAccessType::Internal => 1,
            },

            // Unmapped/unknown regions - assume 1 cycle
            _ => 1,
        }
    }

    /// Calculate cycles for instruction fetch
    pub fn instruction_fetch_cycles(pc: u32, is_thumb: bool) -> CycleInfo {
        let width = if is_thumb { 16 } else { 32 };
        let cycles = Self::get_access_cycles(pc, MemoryAccessType::Sequential, width);
        CycleInfo::sequential(cycles)
    }

    /// Calculate cycles for data access (load/store)
    pub fn data_access_cycles(addr: u32, is_sequential: bool, width: u32) -> CycleInfo {
        let access_type = if is_sequential {
            MemoryAccessType::Sequential
        } else {
            MemoryAccessType::NonSequential
        };
        let cycles = Self::get_access_cycles(addr, access_type, width);
        CycleInfo::new(cycles, access_type)
    }
}

/// Instruction cycle costs (base cycles + memory access cycles)
pub struct InstructionTiming;

impl InstructionTiming {
    /// Get base CPU cycles for different instruction types (excluding memory access)
    pub fn get_base_cycles(opcode: &crate::arm7tdmi::decoder::Opcode, _is_thumb: bool) -> u32 {
        use crate::arm7tdmi::decoder::Opcode;

        match opcode {
            // Data processing instructions
            Opcode::Add
            | Opcode::Adc
            | Opcode::Sub
            | Opcode::Sbc
            | Opcode::Rsb
            | Opcode::Rsc
            | Opcode::And
            | Opcode::Orr
            | Opcode::Eor
            | Opcode::Bic
            | Opcode::Mov
            | Opcode::Mvn
            | Opcode::Cmp
            | Opcode::Cmn
            | Opcode::Tst
            | Opcode::Teq => 1,

            // Shifts (when used as instructions)
            Opcode::Lsl | Opcode::Lsr | Opcode::Asr | Opcode::Ror => 1,

            // Multiply instructions
            Opcode::Mul | Opcode::Mla => {
                // Multiply takes 2-5 cycles depending on operand value
                // For simplicity, using average of 3 cycles
                3
            }

            // Long multiply instructions
            Opcode::Umull | Opcode::Umlal | Opcode::Smull | Opcode::Smlal => {
                // Long multiply takes 3-6 cycles
                // For simplicity, using average of 4 cycles
                4
            }

            // Branch instructions
            Opcode::B => 3,  // 2S + 1N cycles
            Opcode::Bl => 3, // 2S + 1N cycles
            Opcode::Bx => 3, // 2S + 1N cycles

            // Load/Store instructions (base cycles, memory access calculated separately)
            Opcode::Ldr | Opcode::Str => 1,
            Opcode::Ldm | Opcode::Stm => 1, // +nS for each register

            // PSR transfer
            Opcode::Mrs | Opcode::Msr => 1,

            // Software interrupt
            Opcode::Swi => 3, // 2S + 1N cycles

            // Stack operations (THUMB)
            Opcode::Push | Opcode::Pop => 1, // +nS for each register

            // THUMB-specific
            Opcode::Neg => 1,

            // Swap instruction
            Opcode::Swp => 3, // 1N + 1S + 1I cycles
        }
    }
}
