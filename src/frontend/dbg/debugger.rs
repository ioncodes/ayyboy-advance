use super::widgets::cpu::CpuWidget;
use super::widgets::disasm::DisassemblyWidget;
use super::widgets::memory::MemoryWidget;
use super::widgets::ppu::PpuWidget;
use crate::frontend::event::{RequestEvent, ResponseEvent};
use crossbeam_channel::{Receiver, Sender};
use egui::Context;

pub struct Debugger {
    pub open: bool,
    rx: Receiver<ResponseEvent>,
    memory_widget: MemoryWidget,
    cpu_widget: CpuWidget,
    disasm_widget: DisassemblyWidget,
    ppu_widget: PpuWidget,
}

impl Debugger {
    pub fn new(
        cpu_tx: Sender<RequestEvent>, memory_tx: Sender<RequestEvent>, disasm_tx: Sender<RequestEvent>,
        ppu_tx: Sender<RequestEvent>, rx: Receiver<ResponseEvent>,
    ) -> Debugger {
        Debugger {
            open: false,
            rx,
            memory_widget: MemoryWidget::new(memory_tx),
            cpu_widget: CpuWidget::new(cpu_tx),
            disasm_widget: DisassemblyWidget::new(disasm_tx),
            ppu_widget: PpuWidget::new(ppu_tx),
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        match self.rx.try_recv() {
            Ok(ResponseEvent::Cpu(cpu)) => self.cpu_widget.update(cpu),
            Ok(ResponseEvent::Memory(memory)) => self.memory_widget.update(memory),
            Ok(ResponseEvent::Disassembly(pc, r15, disassembly)) => self.disasm_widget.update(disassembly, pc, r15),
            Ok(ResponseEvent::Ppu(frames)) => self.ppu_widget.update(frames),
            _ => (),
        }

        self.cpu_widget.render(ctx);
        self.memory_widget.render(ctx);
        self.disasm_widget.render(ctx);
        self.ppu_widget.render(ctx);
    }

    pub fn toggle_window(&mut self) {
        self.open = !self.open;
    }
}
