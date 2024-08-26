use crate::frontend::event::RequestEvent;
use crossbeam_channel::Sender;
use egui::{Color32, Context, RichText, ScrollArea, Window};

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
            ui.vertical(|ui| {
                ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                    for line in self.disassembly.iter() {
                        ui.horizontal(|ui| {
                            let mut addr_label = RichText::new(format!("{:08x}", line.addr)).monospace().strong();
                            let mut instr_label = RichText::new(line.instr.clone()).monospace();
                            if line.addr == self.pc {
                                addr_label = addr_label.color(Color32::from_rgba_premultiplied(193, 225, 193, 255));
                                instr_label = instr_label.color(Color32::from_rgba_premultiplied(193, 225, 193, 255));
                            } else if line.addr == self.r15 {
                                addr_label = addr_label.color(Color32::from_rgba_premultiplied(195, 177, 225, 255));
                                instr_label = instr_label.color(Color32::from_rgba_premultiplied(195, 177, 225, 255));
                            }
                            ui.label(addr_label);
                            ui.label(instr_label);
                        });
                    }
                });
            });
        });
    }
}
