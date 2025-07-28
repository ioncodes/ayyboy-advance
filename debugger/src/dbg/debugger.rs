use super::widgets::cpu::CpuWidget;
use super::widgets::disasm::DisassemblyWidget;
use super::widgets::memory::MemoryWidget;
use super::widgets::ppu::PpuWidget;
use crate::event::{RequestEvent, ResponseEvent};
use crossbeam_channel::{Receiver, Sender};
use egui::{CentralPanel, Context, SidePanel};

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

    pub fn update_data(&mut self, ctx: &Context) {
        if !self.open {
            return;
        }

        match self.rx.try_recv() {
            Ok(ResponseEvent::Cpu(cpu)) => self.cpu_widget.update(cpu),
            Ok(ResponseEvent::Memory(memory)) => self.memory_widget.update(memory),
            Ok(ResponseEvent::Disassembly(pc, r15, disassembly)) => self.disasm_widget.update(disassembly, pc, r15),
            Ok(ResponseEvent::Ppu(frames, _tileset, tilemaps, palette, registers, sprites)) => {
                // TODO: we ignore tileset cause its been causing issues
                self.ppu_widget
                    .update(ctx, frames, tilemaps, palette, registers, sprites)
            }
            _ => (),
        }
    }

    pub fn render_tiled_debugger(&mut self, screen_texture: &egui::TextureHandle, ctx: &Context) {
        // Left sidebar - CPU and Disassembly
        SidePanel::left("cpu_disasm_panel")
            .resizable(true)
            .default_width(400.0)
            .min_width(300.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("CPU & Control");
                    self.cpu_widget.render_content(ui);

                    ui.separator();

                    ui.heading("Disassembly");
                    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                        self.disasm_widget.render_content(ui);
                    });
                });
            });

        // Far right sidebar - PPU Video only
        SidePanel::right("ppu_video_panel")
            .resizable(true)
            .default_width(350.0)
            .min_width(300.0)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.heading("PPU Video");
                    self.ppu_widget.render_video_content(ui);
                });
            });

        // PPU Registers sidebar - between center and right
        SidePanel::right("ppu_registers_panel")
            .resizable(true)
            .default_width(300.0)
            .min_width(250.0)
            .show(ctx, |ui| {
                ui.heading("PPU Registers");
                egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                    self.ppu_widget.render_registers_content(ui);
                });
            });

        // Central panel - Memory above Game screen
        CentralPanel::default()
            .frame(
                egui::Frame::default()
                    .inner_margin(egui::Margin::symmetric(8, 0))
                    .fill(ctx.style().visuals.panel_fill),
            )
            .show(ctx, |ui| {
                // Memory section (top 60% of center panel)
                ui.heading("Memory");
                let total_height = ui.available_height();
                let memory_height = total_height * 0.6;

                ui.allocate_ui_with_layout(
                    egui::vec2(ui.available_width(), memory_height),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        self.memory_widget.render_content(ui);
                    },
                );

                ui.separator();

                // Game Screen section (bottom 40% of center panel)
                ui.heading("Game Screen");

                // Center the screen in the remaining space
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    let available_size = ui.available_size();
                    let aspect_ratio = 240.0 / 160.0; // GBA screen aspect ratio
                    let max_width = available_size.x;
                    let max_height = available_size.y;

                    let (width, height) = if max_width / max_height > aspect_ratio {
                        // Limited by height
                        (max_height * aspect_ratio, max_height)
                    } else {
                        // Limited by width
                        (max_width, max_width / aspect_ratio)
                    };

                    let image = egui::Image::new(screen_texture)
                        .fit_to_exact_size(egui::vec2(width, height))
                        .texture_options(egui::TextureOptions::NEAREST);
                    ui.add(image);
                });
            });
    }

    pub fn toggle_window(&mut self) {
        self.open = !self.open;
    }
}
