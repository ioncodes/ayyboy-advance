use egui::Context;
use tokio::sync::watch::Receiver;

use super::state::DbgState;
use super::widgets::cpu::CpuWidget;
use super::widgets::memory::MemoryWidget;

pub struct Debugger {
    pub open: bool,
    rx: Receiver<DbgState>,
    last_state: DbgState,
    memory_widget: MemoryWidget,
    cpu_widget: CpuWidget,
}

impl Debugger {
    pub fn new(rx: Receiver<DbgState>) -> Self {
        Self {
            open: false,
            rx,
            last_state: DbgState::default(),
            memory_widget: MemoryWidget::new(),
            cpu_widget: CpuWidget::new(),
        }
    }

    pub fn update(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        match self.rx.has_changed() {
            Ok(true) => {
                let frame = self.rx.borrow_and_update();
                self.last_state = frame.clone();
            }
            _ => {}
        }

        self.cpu_widget.render(ctx, &self.last_state);
        self.memory_widget.render(ctx, &self.last_state);
    }

    pub fn toggle_window(&mut self) {
        self.open = !self.open;
    }
}
