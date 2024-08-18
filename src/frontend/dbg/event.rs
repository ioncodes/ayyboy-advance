use super::widgets::cpu::Cpu;
use super::widgets::disasm::DecodedInstruction;

#[derive(Debug)]
pub enum RequestEvent {
    UpdateMemory,
    UpdateCpu,
    UpdateDisassembly(u32, u32),
    Break,
    Run,
    Step,
    AddBreakpoint(u32),
    RemoveBreakpoint(u32),
}

pub enum ResponseEvent {
    Memory(Box<[u8; 0x0FFFFFFF + 1]>),
    Cpu(Cpu),
    Disassembly(Vec<DecodedInstruction>),
}
