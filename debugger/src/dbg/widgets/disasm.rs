use crate::dbg::widgets::{PC_COLOR, R15_COLOR};
use crate::event::RequestEvent;
use crossbeam_channel::Sender;
use egui::{Context, RichText, Window};

pub struct DecodedInstruction {
    pub addr: u32,
    pub instr: String,
}

pub struct DisassemblyWidget {
    event_tx: Sender<RequestEvent>,
    disassembly: Vec<DecodedInstruction>,
    pc: u32,
    r15: u32,
}

impl DisassemblyWidget {
    pub fn new(tx: Sender<RequestEvent>) -> DisassemblyWidget {
        let _ = tx.send(RequestEvent::UpdateDisassembly(None, 25)); // request initial disassembly

        DisassemblyWidget {
            event_tx: tx,
            disassembly: Vec::new(),
            pc: 0,
            r15: 0,
        }
    }

    pub fn update(&mut self, disassembly: Vec<DecodedInstruction>, pc: u32, r15: u32) {
        self.disassembly = disassembly;
        self.pc = pc;
        self.r15 = r15;
        let _ = self.event_tx.send(RequestEvent::UpdateDisassembly(None, 25));
    }

    pub fn render(&mut self, ctx: &Context) {
        Window::new("Disassembly").resizable(false).show(ctx, |ui| {
            self.render_content(ui);
        });
    }

    pub fn render_content(&mut self, ui: &mut egui::Ui) {
        for line in self.disassembly.iter() {
            ui.horizontal(|ui| {
                let mut addr_label = RichText::new(format!("{:08X}", line.addr)).monospace().strong();
                let mut instr_label = RichText::new(line.instr.clone()).monospace();
                if line.addr == self.pc {
                    addr_label = addr_label.color(PC_COLOR);
                    instr_label = instr_label.color(PC_COLOR);
                } else if line.addr == self.r15 {
                    addr_label = addr_label.color(R15_COLOR);
                    instr_label = instr_label.color(R15_COLOR);
                }
                ui.label(addr_label);
                ui.label(instr_label);
            });
        }
    }
}
