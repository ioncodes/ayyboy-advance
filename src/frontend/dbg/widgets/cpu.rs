use crate::arm7tdmi;
use crate::frontend::dbg::event::{RequestEvent, ResponseEvent};
use crossbeam_channel::{Receiver, Sender};
use egui::{Context, RichText, Window};

#[derive(Default, Copy, Clone)]
pub struct Cpu {
    pub registers: [u32; 16],
    pub cpsr: arm7tdmi::registers::Psr,
}

pub struct CpuWidget {
    event_rx: Receiver<ResponseEvent>,
    event_tx: Sender<RequestEvent>,
    cpu: Cpu,
}

impl CpuWidget {
    pub fn new(rx: Receiver<ResponseEvent>, tx: Sender<RequestEvent>) -> CpuWidget {
        CpuWidget {
            event_rx: rx,
            event_tx: tx,
            cpu: Cpu::default(),
        }
    }

    pub fn update(&mut self) {
        match self.event_rx.try_recv() {
            Ok(ResponseEvent::Cpu(cpu)) => self.cpu = cpu,
            _ => (),
        }

        let _ = self.event_tx.send(RequestEvent::UpdateCpu);
    }

    pub fn render(&self, ctx: &Context) {
        Window::new("CPU").resizable(false).show(ctx, |ui| {
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
