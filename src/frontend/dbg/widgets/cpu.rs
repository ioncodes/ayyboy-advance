use crate::arm7tdmi;
use crate::frontend::dbg::event::RequestEvent;
use crossbeam_channel::Sender;
use egui::{ComboBox, Context, RichText, TextEdit, Vec2, Window};

#[derive(Default, Copy, Clone)]
pub struct Cpu {
    pub registers: [u32; 16],
    pub cpsr: arm7tdmi::registers::Psr,
}

pub struct CpuWidget {
    pub cpu: Cpu,
    event_tx: Sender<RequestEvent>,
    breakpoint: String,
    selected_breakpoint: String,
    breakpoints: Vec<String>,
}

impl CpuWidget {
    pub fn new(tx: Sender<RequestEvent>) -> CpuWidget {
        let _ = tx.send(RequestEvent::UpdateCpu); // request initial CPU state

        CpuWidget {
            event_tx: tx,
            cpu: Cpu::default(),
            breakpoint: String::new(),
            selected_breakpoint: String::new(),
            breakpoints: Vec::new(),
        }
    }

    pub fn update(&mut self, cpu: Cpu) {
        self.cpu = cpu;
        let _ = self.event_tx.send(RequestEvent::UpdateCpu);
    }

    pub fn render(&mut self, ctx: &Context) {
        Window::new("CPU").resizable(false).max_width(100.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(format!("{} Run", egui_phosphor::regular::PLAY)).clicked() {
                    let _ = self.event_tx.send(RequestEvent::Run);
                }

                if ui.button(format!("{} Step", egui_phosphor::regular::STEPS)).clicked() {
                    let _ = self.event_tx.send(RequestEvent::Step);
                }

                if ui.button(format!("{} Break", egui_phosphor::regular::PAUSE)).clicked() {
                    let _ = self.event_tx.send(RequestEvent::Break);
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                TextEdit::singleline(&mut self.breakpoint)
                    .hint_text("Breakpoint")
                    .min_size(Vec2::new(249.0, 0.0))
                    .show(ui);

                if ui
                    .button(format!("{} Add Breakpoint", egui_phosphor::regular::BUG))
                    .clicked()
                {
                    self.breakpoints.push(self.breakpoint.clone());
                    let _ = self.event_tx.send(RequestEvent::AddBreakpoint(
                        u32::from_str_radix(&self.breakpoint, 16).unwrap(),
                    ));
                }
            });

            ui.horizontal(|ui| {
                ComboBox::from_label("Breakpoints")
                    .selected_text(format!("{}", self.selected_breakpoint))
                    .width(175.0)
                    .show_ui(ui, |ui| {
                        for breakpoint in &self.breakpoints {
                            ui.selectable_value(&mut self.selected_breakpoint, breakpoint.to_owned(), breakpoint);
                        }
                    });

                if ui
                    .button(format!("{} Delete Breakpoint", egui_phosphor::regular::TRASH))
                    .clicked()
                {
                    self.breakpoints.retain(|x| x != &self.breakpoint);
                    let _ = self.event_tx.send(RequestEvent::RemoveBreakpoint(
                        u32::from_str_radix(&self.breakpoint, 16).unwrap(),
                    ));
                }
            });

            ui.separator();

            ui.horizontal(|ui| {
                ui.label(RichText::new(format!(" R0: {:08x}", self.cpu.registers[0])).monospace());
                ui.label(RichText::new(format!(" R1: {:08x}", self.cpu.registers[1])).monospace());
                ui.label(RichText::new(format!(" R2: {:08x}", self.cpu.registers[2])).monospace());
                ui.label(RichText::new(format!(" R3: {:08x}", self.cpu.registers[3])).monospace());
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!(" R4: {:08x}", self.cpu.registers[4])).monospace());
                ui.label(RichText::new(format!(" R5: {:08x}", self.cpu.registers[5])).monospace());
                ui.label(RichText::new(format!(" R6: {:08x}", self.cpu.registers[6])).monospace());
                ui.label(RichText::new(format!(" R7: {:08x}", self.cpu.registers[7])).monospace());
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!(" R8: {:08x}", self.cpu.registers[8])).monospace());
                ui.label(RichText::new(format!(" R9: {:08x}", self.cpu.registers[9])).monospace());
                ui.label(RichText::new(format!("R10: {:08x}", self.cpu.registers[10])).monospace());
                ui.label(RichText::new(format!("R11: {:08x}", self.cpu.registers[11])).monospace());
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new(format!("R12: {:08x}", self.cpu.registers[12])).monospace());
                ui.label(RichText::new(format!("R13: {:08x}", self.cpu.registers[13])).monospace());
                ui.label(RichText::new(format!("R14: {:08x}", self.cpu.registers[14])).monospace());
                ui.label(RichText::new(format!("R15: {:08x}", self.cpu.registers[15])).monospace());
            });
            ui.label(RichText::new(format!("CPSR: {:032b} ({})", self.cpu.cpsr, self.cpu.cpsr)).monospace());
        });
    }
}
