use std::fmt::Display;

use bitflags::bitflags;

bitflags! {
    #[derive(Default)]
    pub struct Cpsr: u32 {
        const N = 1 << 31;
        const Z = 1 << 30;
        const C = 1 << 29;
        const V = 1 << 28;
        const Q = 1 << 27;
        const J = 1 << 24;
        const GE = 0b1111 << 16;
        const E = 1 << 9;
        const A = 1 << 8;
        const I = 1 << 7;
        const F = 1 << 6;
        const T = 1 << 5;
        const M = 0b11111;
    }
}

bitflags! {
    #[derive(Default)]
    pub struct Spsr: u32 {
        const N = 1 << 31;
        const Z = 1 << 30;
        const C = 1 << 29;
        const V = 1 << 28;
        const Q = 1 << 27;
        const J = 1 << 24;
        const GE = 0b1111 << 16;
        const E = 1 << 9;
        const A = 1 << 8;
        const I = 1 << 7;
        const F = 1 << 6;
        const T = 1 << 5;
        const M = 0b11111;
    }

}

impl Display for Cpsr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.contains(Cpsr::N) {
            write!(f, "N")?;
        } else {
            write!(f, "n")?;
        }

        if self.contains(Cpsr::Z) {
            write!(f, "Z")?;
        } else {
            write!(f, "z")?;
        }

        if self.contains(Cpsr::C) {
            write!(f, "C")?;
        } else {
            write!(f, "c")?;
        }

        if self.contains(Cpsr::V) {
            write!(f, "V")?;
        } else {
            write!(f, "v")?;
        }

        if self.contains(Cpsr::Q) {
            write!(f, "Q")?;
        } else {
            write!(f, "q")?;
        }

        if self.contains(Cpsr::J) {
            write!(f, "J")?;
        } else {
            write!(f, "j")?;
        }

        if self.contains(Cpsr::GE) {
            write!(f, "G")?;
        } else {
            write!(f, "g")?;
        }

        if self.contains(Cpsr::E) {
            write!(f, "E")?;
        } else {
            write!(f, "e")?;
        }

        if self.contains(Cpsr::A) {
            write!(f, "A")?;
        } else {
            write!(f, "a")?;
        }

        if self.contains(Cpsr::I) {
            write!(f, "I")?;
        } else {
            write!(f, "i")?;
        }

        if self.contains(Cpsr::F) {
            write!(f, "F")?;
        } else {
            write!(f, "f")?;
        }

        if self.contains(Cpsr::T) {
            write!(f, "T")?;
        } else {
            write!(f, "t")?;
        }

        Ok(())
    }
}

impl Display for Spsr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.contains(Spsr::N) {
            write!(f, "N")?;
        } else {
            write!(f, "n")?;
        }

        if self.contains(Spsr::Z) {
            write!(f, "Z")?;
        } else {
            write!(f, "z")?;
        }

        if self.contains(Spsr::C) {
            write!(f, "C")?;
        } else {
            write!(f, "c")?;
        }

        if self.contains(Spsr::V) {
            write!(f, "V")?;
        } else {
            write!(f, "v")?;
        }

        if self.contains(Spsr::Q) {
            write!(f, "Q")?;
        } else {
            write!(f, "q")?;
        }

        if self.contains(Spsr::J) {
            write!(f, "J")?;
        } else {
            write!(f, "j")?;
        }

        if self.contains(Spsr::GE) {
            write!(f, "G")?;
        } else {
            write!(f, "g")?;
        }

        if self.contains(Spsr::E) {
            write!(f, "E")?;
        } else {
            write!(f, "e")?;
        }

        if self.contains(Spsr::A) {
            write!(f, "A")?;
        } else {
            write!(f, "a")?;
        }

        if self.contains(Spsr::I) {
            write!(f, "I")?;
        } else {
            write!(f, "i")?;
        }

        if self.contains(Spsr::F) {
            write!(f, "F")?;
        } else {
            write!(f, "f")?;
        }

        if self.contains(Spsr::T) {
            write!(f, "T")?;
        } else {
            write!(f, "t")?;
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct Registers {
    pub r: [u32; 16],
    pub cpsr: Cpsr,
    pub spsr: [Spsr; 5],
}
