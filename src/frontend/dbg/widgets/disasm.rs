use crate::frontend::dbg::event::RequestEvent;
use crossbeam_channel::Sender;
use egui::{Context, RichText, ScrollArea, Window};

pub struct DecodedInstruction {
    pub addr: u32,
    pub instr: String,
}

pub struct DisassemblyWidget {
    event_tx: Sender<RequestEvent>,
    disassembly: Vec<DecodedInstruction>,
}

impl DisassemblyWidget {
    pub fn new(tx: Sender<RequestEvent>) -> DisassemblyWidget {
        let _ = tx.send(RequestEvent::UpdateDisassembly(None, 25)); // request initial disassembly

        DisassemblyWidget {
            event_tx: tx,
            disassembly: Vec::new(),
        }
    }

    pub fn update(&mut self, disassembly: Vec<DecodedInstruction>) {
        self.disassembly = disassembly;
        let _ = self.event_tx.send(RequestEvent::UpdateDisassembly(None, 25));
    }

    pub fn render(&mut self, ctx: &Context) {
        Window::new("Disassembly").resizable(false).show(ctx, |ui| {
            ui.vertical(|ui| {
                ScrollArea::vertical().show(ui, |ui| {
                    for line in self.disassembly.iter() {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("{:08x}", line.addr)).monospace().strong());
                            ui.label(RichText::new(line.instr.clone()).monospace());
                        });
                    }
                });
            });
        });
    }
}
