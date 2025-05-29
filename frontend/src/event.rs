use gba_core::input::registers::KeyInput;
use gba_core::video::registers::InternalScreenSize;
use gba_core::video::{Frame, Pixel};

use crate::dbg::widgets::ppu::PpuRegisters;

use super::dbg::widgets::cpu::Cpu;
use super::dbg::widgets::disasm::DecodedInstruction;

#[derive(Debug)]
pub enum RequestEvent {
    UpdateMemory,
    UpdateCpu,
    UpdatePpu,
    UpdateDisassembly(Option<u32>, u32),
    Break,
    Run,
    Step,
    AddBreakpoint(u32),
    RemoveBreakpoint(u32),
    UpdateKeyState(Vec<(KeyInput, bool)>),
}

pub enum ResponseEvent {
    Memory(Box<[u8; 0x0FFFFFFF + 1]>),
    Cpu(Cpu),
    Disassembly(u32, u32, Vec<DecodedInstruction>),
    Ppu(
        Vec<Frame>,
        (usize, Vec<Pixel>),
        [(InternalScreenSize, Vec<Pixel>); 4],
        Vec<Pixel>,
        PpuRegisters,
    ), // TODO: BG Mode 3,4,5 each frame 0 and 1
}
