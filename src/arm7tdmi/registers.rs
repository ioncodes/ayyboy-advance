use super::cpu::ProcessorMode;
use bitflags::bitflags;
use std::fmt::Display;

bitflags! {
    #[derive(Copy, Clone, Default)]
    pub struct Psr: u32 {
        const N = 1 << 31;
        const Z = 1 << 30;
        const C = 1 << 29;
        const V = 1 << 28;
        // ... Reserved for future revisions
        const I = 1 << 7;
        const F = 1 << 6;
        const T = 1 << 5;
        const M = 0b11111;
    }
}

impl Display for Psr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.contains(Psr::N) {
            write!(f, "N")?;
        } else {
            write!(f, "n")?;
        }

        if self.contains(Psr::Z) {
            write!(f, "Z")?;
        } else {
            write!(f, "z")?;
        }

        if self.contains(Psr::C) {
            write!(f, "C")?;
        } else {
            write!(f, "c")?;
        }

        if self.contains(Psr::V) {
            write!(f, "V")?;
        } else {
            write!(f, "v")?;
        }

        if self.contains(Psr::I) {
            write!(f, "I")?;
        } else {
            write!(f, "i")?;
        }

        if self.contains(Psr::F) {
            write!(f, "F")?;
        } else {
            write!(f, "f")?;
        }

        if self.contains(Psr::T) {
            write!(f, "T")?;
        } else {
            write!(f, "t")?;
        }

        Ok(())
    }
}

pub struct Registers {
    pub r: [u32; 16],
    pub cpsr: Psr,
    pub spsr: [Psr; 5],
}

impl Default for Registers {
    fn default() -> Self {
        Self {
            r: [0; 16],
            cpsr: Psr::from_bits_truncate(ProcessorMode::Supervisor.into()),
            spsr: [Psr::from_bits_truncate(0); 5],
        }
    }
}
