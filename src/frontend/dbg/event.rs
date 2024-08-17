use super::widgets::cpu::Cpu;

#[derive(Clone)]
pub enum RequestEvent {
    UpdateMemory,
    UpdateCpu,
    None,
}

#[derive(Clone)]
pub enum ResponseEvent {
    Memory(Box<[u8; 0x0FFFFFFF + 1]>),
    Cpu(Cpu),
    None,
}

impl Default for RequestEvent {
    fn default() -> RequestEvent {
        RequestEvent::None
    }
}

impl Default for ResponseEvent {
    fn default() -> ResponseEvent {
        ResponseEvent::None
    }
}
