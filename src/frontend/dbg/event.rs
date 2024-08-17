use super::widgets::cpu::Cpu;

pub enum RequestEvent {
    UpdateMemory,
    UpdateCpu,
}

pub enum ResponseEvent {
    Memory(Box<[u8; 0x0FFFFFFF + 1]>),
    Cpu(Cpu),
}
