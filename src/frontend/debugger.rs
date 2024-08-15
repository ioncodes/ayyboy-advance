use egui::{Context, RichText, Window};
use tokio::sync::watch::Receiver;

use super::state::DbgState;

pub struct Debugger {
    pub open: bool,
    rx: Receiver<DbgState>,
    last_state: DbgState,
}

impl Debugger {
    pub fn new(rx: Receiver<DbgState>) -> Self {
        Self {
            open: false,
            rx,
            last_state: DbgState::default(),
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        match self.rx.has_changed() {
            Ok(true) => {
                let frame = self.rx.borrow_and_update();
                self.last_state = *frame;
            }
            _ => {}
        }

        Window::new("CPU").open(&mut self.open).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!(" R0: {:08x}", self.last_state.cpu.registers[0])).monospace());
                ui.label(RichText::new(format!(" R1: {:08x}", self.last_state.cpu.registers[1])).monospace());
                ui.label(RichText::new(format!(" R2: {:08x}", self.last_state.cpu.registers[2])).monospace());
                ui.label(RichText::new(format!(" R3: {:08x}", self.last_state.cpu.registers[3])).monospace());
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!(" R4: {:08x}", self.last_state.cpu.registers[4])).monospace());
                ui.label(RichText::new(format!(" R5: {:08x}", self.last_state.cpu.registers[5])).monospace());
                ui.label(RichText::new(format!(" R6: {:08x}", self.last_state.cpu.registers[6])).monospace());
                ui.label(RichText::new(format!(" R7: {:08x}", self.last_state.cpu.registers[7])).monospace());
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!(" R8: {:08x}", self.last_state.cpu.registers[8])).monospace());
                ui.label(RichText::new(format!(" R9: {:08x}", self.last_state.cpu.registers[9])).monospace());
                ui.label(RichText::new(format!("R10: {:08x}", self.last_state.cpu.registers[10])).monospace());
                ui.label(RichText::new(format!("R11: {:08x}", self.last_state.cpu.registers[11])).monospace());
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("R12: {:08x}", self.last_state.cpu.registers[12])).monospace());
                ui.label(RichText::new(format!("R13: {:08x}", self.last_state.cpu.registers[13])).monospace());
                ui.label(RichText::new(format!("R14: {:08x}", self.last_state.cpu.registers[14])).monospace());
                ui.label(RichText::new(format!("R15: {:08x}", self.last_state.cpu.registers[15])).monospace());
            });
            ui.label(
                RichText::new(format!(
                    "CPSR: {:032b} ({})",
                    self.last_state.cpu.cpsr, self.last_state.cpu.cpsr
                ))
                .monospace(),
            );
        });
    }

    pub fn toggle_window(&mut self) {
        self.open = !self.open;
    }
}
