use log::error;
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Hash, Eq)]
pub enum ProcessorMode {
    User = 0b10000,
    Fiq = 0b10001,
    Irq = 0b10010,
    Supervisor = 0b10011,
    Abort = 0b10111,
    System = 0b11111,
    Undefined = 0b11011,
    Invalid = 0b00000,
}

impl ProcessorMode {
    pub fn from(value: u32) -> ProcessorMode {
        match value {
            0b10000 => ProcessorMode::User,
            0b10001 => ProcessorMode::Fiq,
            0b10010 => ProcessorMode::Irq,
            0b10011 => ProcessorMode::Supervisor,
            0b10111 => ProcessorMode::Abort,
            0b11011 => ProcessorMode::Undefined,
            0b11111 => ProcessorMode::System,
            _ => {
                error!("Invalid processor mode: {:08b}", value);
                ProcessorMode::Invalid
            }
        }
    }
}

impl Display for ProcessorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessorMode::User => write!(f, "User"),
            ProcessorMode::Fiq => write!(f, "Fiq"),
            ProcessorMode::Irq => write!(f, "Irq"),
            ProcessorMode::Supervisor => write!(f, "Supervisor"),
            ProcessorMode::Abort => write!(f, "Abort"),
            ProcessorMode::System => write!(f, "System"),
            ProcessorMode::Undefined => write!(f, "Undefined"),
            ProcessorMode::Invalid => write!(f, "Invalid"),
        }
    }
}
