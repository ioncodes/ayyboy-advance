use super::cpu::ProcessorMode;
use bitflags::bitflags;
use std::collections::HashMap;
use std::fmt::Display;
use std::hash::Hash;

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
    pub banked: HashMap<ProcessorMode, Vec<u32>>,
}

impl Default for Registers {
    fn default() -> Self {
        let mut banked = HashMap::new();
        banked.insert(ProcessorMode::Fiq, Vec::new());
        banked.insert(ProcessorMode::Irq, Vec::new());
        banked.insert(ProcessorMode::Supervisor, Vec::new());
        banked.insert(ProcessorMode::Abort, Vec::new());
        banked.insert(ProcessorMode::Undefined, Vec::new());

        Self {
            r: [0; 16],
            cpsr: Psr::from_bits_truncate(ProcessorMode::Supervisor as u32),
            spsr: [
                Psr::from_bits_truncate(ProcessorMode::Fiq as u32),
                Psr::from_bits_truncate(ProcessorMode::Supervisor as u32),
                Psr::from_bits_truncate(ProcessorMode::Abort as u32),
                Psr::from_bits_truncate(ProcessorMode::Irq as u32),
                Psr::from_bits_truncate(ProcessorMode::Undefined as u32),
            ],
            banked,
        }
    }
}
