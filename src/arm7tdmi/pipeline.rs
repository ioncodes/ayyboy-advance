use super::decoder::Instruction;
use crate::memory::mmio::Mmio;
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
}

impl Pipeline {
    pub fn new() -> Pipeline {
        Pipeline {
            fetch: None,
            decode: None,
        }
    }

    pub fn advance(&mut self, pc: u32, is_thumb: bool, mmio: &Mmio) {
        self.decode = if let Some(Item::Data(opcode, state)) = self.fetch.take() {
            let instruction = match Instruction::decode(opcode, is_thumb) {
                Ok(instr) => Some(instr),
                Err(e) => panic!("{}", e),
            };
            Some(Item::Instruction(instruction.unwrap(), state))
        } else {
            None
        };

        let opcode = mmio.read_u32(pc);
        self.fetch = Some(Item::Data(opcode, State { pc, opcode }));
    }

    pub fn pop(&mut self) -> Option<(Instruction, State)> {
        match self.decode.take() {
            Some(Item::Instruction(instruction, state)) => Some((instruction, state)),
            Some(Item::Data(_, _)) => panic!("Data found in decode stage"),
            _ => None,
        }
    }

    pub fn flush(&mut self) {
        self.fetch = None;
        self.decode = None;
    }

    pub fn peek_fetch(&self) -> Option<(&u32, &State)> {
        match &self.fetch {
            Some(Item::Data(data, state)) => Some((data, state)),
            _ => None,
        }
    }
}

impl Display for Pipeline {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Fetch = {{{}}}, Decode = {{{}}}",
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
        )
    }
}
