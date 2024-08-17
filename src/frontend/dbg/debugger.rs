use super::event::{RequestEvent, ResponseEvent};
use super::widgets::cpu::CpuWidget;
use super::widgets::memory::MemoryWidget;
use crossbeam_channel::{Receiver, Sender};
use egui::Context;

pub struct Debugger {
    pub open: bool,
    memory_widget: MemoryWidget,
    cpu_widget: CpuWidget,
}

impl Debugger {
    pub fn new(
        cpu_tx: Sender<RequestEvent>, cpu_rx: Receiver<ResponseEvent>, memory_tx: Sender<RequestEvent>,
        memory_rx: Receiver<ResponseEvent>,
    ) -> Debugger {
        Self {
            open: false,
            memory_widget: MemoryWidget::new(memory_rx, memory_tx),
            cpu_widget: CpuWidget::new(cpu_rx, cpu_tx),
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        self.cpu_widget.update();
        self.memory_widget.update();

        self.cpu_widget.render(ctx);
        self.memory_widget.render(ctx);
    }

    pub fn toggle_window(&mut self) {
        self.open = !self.open;
    }
}
