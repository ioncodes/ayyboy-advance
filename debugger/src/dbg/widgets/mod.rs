use egui::Color32;

pub mod cpu;
pub mod disasm;
pub mod memory;
pub mod ppu;

const DIRTY_COLOR: Color32 = Color32::from_rgba_premultiplied(250, 160, 160, 255);
const PC_COLOR: Color32 = Color32::from_rgba_premultiplied(193, 225, 193, 255);
const R15_COLOR: Color32 = Color32::from_rgba_premultiplied(195, 177, 225, 255);
const TRANSPARENT_COLOR: Color32 = Color32::from_rgba_premultiplied(255, 192, 203, 255);
