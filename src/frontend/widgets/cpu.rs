use crate::frontend::state::DbgState;
use egui::{Context, RichText, Window};

pub struct CpuWidget {}

impl CpuWidget {
    pub fn new() -> CpuWidget {
        CpuWidget {}
    }

    pub fn render(&self, ctx: &Context, state: &DbgState) {
        Window::new("CPU").resizable(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!(" R0: {:08x}", state.cpu.registers[0])).monospace());
                ui.label(RichText::new(format!(" R1: {:08x}", state.cpu.registers[1])).monospace());
                ui.label(RichText::new(format!(" R2: {:08x}", state.cpu.registers[2])).monospace());
                ui.label(RichText::new(format!(" R3: {:08x}", state.cpu.registers[3])).monospace());
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!(" R4: {:08x}", state.cpu.registers[4])).monospace());
                ui.label(RichText::new(format!(" R5: {:08x}", state.cpu.registers[5])).monospace());
                ui.label(RichText::new(format!(" R6: {:08x}", state.cpu.registers[6])).monospace());
                ui.label(RichText::new(format!(" R7: {:08x}", state.cpu.registers[7])).monospace());
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!(" R8: {:08x}", state.cpu.registers[8])).monospace());
                ui.label(RichText::new(format!(" R9: {:08x}", state.cpu.registers[9])).monospace());
                ui.label(RichText::new(format!("R10: {:08x}", state.cpu.registers[10])).monospace());
                ui.label(RichText::new(format!("R11: {:08x}", state.cpu.registers[11])).monospace());
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("R12: {:08x}", state.cpu.registers[12])).monospace());
                ui.label(RichText::new(format!("R13: {:08x}", state.cpu.registers[13])).monospace());
                ui.label(RichText::new(format!("R14: {:08x}", state.cpu.registers[14])).monospace());
                ui.label(RichText::new(format!("R15: {:08x}", state.cpu.registers[15])).monospace());
            });
            ui.label(RichText::new(format!("CPSR: {:032b} ({})", state.cpu.cpsr, state.cpu.cpsr)).monospace());
        });
    }
}
