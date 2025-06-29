use super::decoder::Instruction;
use crate::memory::mmio::Mmio;
use log::*;
use std::fmt::Display;

pub struct State {
    pub pc: u32,
    pub opcode: u32,
    pub is_thumb: bool,
}

pub struct Pipeline {
    states: Vec<State>,
}

impl Pipeline {
    pub fn new() -> Pipeline {
        Pipeline {
            states: Vec::with_capacity(3),
        }
    }

    pub fn advance(&mut self, pc: u32, is_thumb: bool, mmio: &mut Mmio) {
        let opcode = mmio.read_u32(pc);
        self.states.push(State { pc, opcode, is_thumb });
    }

    pub fn pop(&mut self) -> Option<(Instruction, State)> {
        if self.states.len() < 3 {
            return None;
        }

        let state = self.states.remove(0);
        let instr = Instruction::decode(state.opcode, state.is_thumb).unwrap_or_else(|e| {
            error!("Failed to decode instruction: {:?} at {:08x}", e, state.pc);
            Instruction::nop()
        });

        Some((instr, state))
    }

    pub fn flush(&mut self) {
        self.states.clear();
    }

    pub fn peek_fetch(&self) -> Option<&State> {
        self.states.last()
    }

    pub fn is_full(&self) -> bool {
        self.states.len() >= 3
    }

    pub fn is_empty(&self) -> bool {
        self.states.is_empty()
    }
}

impl Display for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Pipeline: [")?;
        for (i, state) in self.states.iter().enumerate() {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:08x} @ {:08x}", state.opcode, state.pc)?;
        }
        write!(f, "]")
    }
}
