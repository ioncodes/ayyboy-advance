use super::decoder::Instruction;
use crate::memory::mmio::Mmio;
use log::*;
use std::fmt::Display;

pub struct State {
    pub pc: u32,
    pub opcode: u32,
}

enum Item {
    Instruction(Instruction, State),
    Data(u32, State),
}

pub struct Pipeline {
    fetch: Option<Item>,
    decode: Option<Item>,
    execute: Option<Item>,
}

impl Pipeline {
    pub fn new() -> Pipeline {
        Pipeline {
            fetch: None,
            decode: None,
            execute: None,
        }
    }

    pub fn advance(&mut self, pc: u32, is_thumb: bool, mmio: &mut Mmio) {
        self.execute = self.decode.take();

        self.decode = self.fetch.take().map(|item| match item {
            Item::Data(opcode, state) => {
                let instr = Instruction::decode(opcode, is_thumb).unwrap_or_else(|e| {
                    error!("Failed to decode instruction: {:?} at {:08x}", e, pc);
                    Instruction::nop()
                });
                Item::Instruction(instr, state)
            }
            Item::Instruction(_, _) => unreachable!(),
        });

        let opcode = mmio.read_u32(pc);
        self.fetch = Some(Item::Data(opcode, State { pc, opcode }));
    }

    pub fn pop(&mut self) -> Option<(Instruction, State)> {
        match self.execute.take()? {
            Item::Instruction(instruction, state) => Some((instruction, state)),
            Item::Data(_, _) => panic!("Data found in decode stage"),
        }
    }

    pub fn flush(&mut self) {
        self.fetch = None;
        self.decode = None;
        self.execute = None;
    }

    pub fn peek_fetch(&self) -> Option<(&u32, &State)> {
        match &self.fetch {
            Some(Item::Data(data, state)) => Some((data, state)),
            _ => None,
        }
    }

    pub fn is_full(&self) -> bool {
        self.fetch.is_some() && self.decode.is_some() && self.execute.is_some()
    }

    pub fn is_empty(&self) -> bool {
        self.fetch.is_none() && self.decode.is_none() && self.execute.is_none()
    }
}

impl Display for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Fetch = {{{}}}, Decode = {{{}}}, Execute = {{{}}}",
            match &self.fetch {
                Some(Item::Instruction(instr, state)) => format!("{} @ {:08x}", instr.opcode, state.pc),
                Some(Item::Data(data, state)) => format!("{:08x} @ {:08x}", data, state.pc),
                None => String::from("Empty"),
            },
            match &self.decode {
                Some(Item::Instruction(instr, state)) => format!("{} @ {:08x}", instr.opcode, state.pc),
                Some(Item::Data(data, state)) => format!("{:08x} @ {:08x}", data, state.pc),
                None => String::from("Empty"),
            },
            match &self.execute {
                Some(Item::Instruction(instr, state)) => format!("{} @ {:08x}", instr.opcode, state.pc),
                Some(Item::Data(data, state)) => format!("{:08x} @ {:08x}", data, state.pc),
                None => String::from("Empty"),
            },
        )
    }
}
