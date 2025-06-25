use crate::dbg::tracked_value::TrackedValue;
use crate::event::RequestEvent;
use crossbeam_channel::Sender;
use egui::{CollapsingHeader, Color32, ComboBox, Context, RichText, TextEdit, Window};
use gba_core::arm7tdmi::registers::Psr;
use gba_core::arm7tdmi::timer::Timers;
use gba_core::memory::dma::Dma;
use gba_core::memory::registers::TimerControl;

#[derive(Default, Copy, Clone)]
pub struct TrackedCpu {
    registers: [TrackedValue<u32>; 16],
    cpsr: TrackedValue<Psr>,
    dma: TrackedValue<Dma>,
    timers: TrackedValue<Timers>,
}

pub struct Cpu {
    pub registers: [u32; 16],
    pub cpsr: Psr,
    pub dma: Dma,
    pub timers: Timers,
}

pub struct CpuWidget {
    pub cpu: TrackedCpu,
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
            cpu: TrackedCpu::default(),
            breakpoint: String::new(),
            selected_breakpoint: String::new(),
            breakpoints: Vec::new(),
        }
    }

    pub fn update(&mut self, cpu: Cpu) {
        self.cpu.registers.iter_mut().enumerate().for_each(|(i, reg)| {
            reg.set(cpu.registers[i]);
        });
        self.cpu.cpsr.set(cpu.cpsr);
        self.cpu.dma.set(cpu.dma);
        self.cpu.timers.set(cpu.timers);
    }

    pub fn render(&mut self, ctx: &Context) {
        Window::new("CPU").resizable(false).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    if ui.button(format!("{} Run", egui_phosphor::regular::PLAY)).clicked() {
                        let _ = self.event_tx.send(RequestEvent::Run);
                    }

                    if ui.button(format!("{} Step", egui_phosphor::regular::STEPS)).clicked() {
                        let _ = self.event_tx.send(RequestEvent::Step);
                        let _ = self.event_tx.send(RequestEvent::UpdateCpu);
                    }

                    if ui.button(format!("{} Break", egui_phosphor::regular::PAUSE)).clicked() {
                        let _ = self.event_tx.send(RequestEvent::Break);
                        let _ = self.event_tx.send(RequestEvent::UpdateCpu);
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui
                        .button(format!("{} Refresh", egui_phosphor::regular::ARROW_CLOCKWISE))
                        .clicked()
                    {
                        let _ = self.event_tx.send(RequestEvent::UpdateCpu);
                    }
                });
            });

            ui.separator();

            ui.horizontal(|ui| {
                if ui
                    .button(format!("{} Add Breakpoint", egui_phosphor::regular::BUG))
                    .clicked()
                {
                    self.breakpoints.push(self.breakpoint.clone());
                    let _ = self.event_tx.send(RequestEvent::AddBreakpoint(
                        u32::from_str_radix(&self.breakpoint, 16).unwrap(),
                    ));
                }

                TextEdit::singleline(&mut self.breakpoint)
                    .hint_text("Breakpoint")
                    .show(ui);
            });

            ui.horizontal(|ui| {
                if ui
                    .button(format!("{} Delete Breakpoint", egui_phosphor::regular::TRASH))
                    .clicked()
                {
                    self.breakpoints.retain(|x| x != &self.breakpoint);
                    let _ = self.event_tx.send(RequestEvent::RemoveBreakpoint(
                        u32::from_str_radix(&self.breakpoint, 16).unwrap(),
                    ));
                }

                ComboBox::from_label("Breakpoints")
                    .selected_text(format!("{}", self.selected_breakpoint))
                    .width(175.0)
                    .show_ui(ui, |ui| {
                        for breakpoint in &self.breakpoints {
                            ui.selectable_value(&mut self.selected_breakpoint, breakpoint.to_owned(), breakpoint);
                        }
                    });
            });

            ui.separator();

            let format_register = |idx: usize| {
                let alignment = if idx <= 9 { " " } else { "" };
                let reg = self.cpu.registers[idx];
                if reg.has_changed() {
                    RichText::new(format!("{}R{}: {:08x}", alignment, idx, reg.get()))
                        .monospace()
                        .color(Color32::from_rgba_premultiplied(250, 160, 160, 255))
                } else {
                    RichText::new(format!("{}R{}: {:08x}", alignment, idx, reg.get())).monospace()
                }
            };

            ui.horizontal(|ui| {
                ui.label(format_register(0));
                ui.label(format_register(1));
                ui.label(format_register(2));
                ui.label(format_register(3));
            });
            ui.horizontal(|ui| {
                ui.label(format_register(4));
                ui.label(format_register(5));
                ui.label(format_register(6));
                ui.label(format_register(7));
            });
            ui.horizontal(|ui| {
                ui.label(format_register(8));
                ui.label(format_register(9));
                ui.label(format_register(10));
                ui.label(format_register(11));
            });
            ui.horizontal(|ui| {
                ui.label(format_register(12));
                ui.label(format_register(13));
                ui.label(format_register(14));
                ui.label(format_register(15));
            });
            ui.label(if self.cpu.cpsr.has_changed() {
                RichText::new(format!("CPSR: {:032b} ({})", self.cpu.cpsr.get(), self.cpu.cpsr.get()))
                    .monospace()
                    .color(Color32::from_rgba_premultiplied(250, 160, 160, 255))
            } else {
                RichText::new(format!("CPSR: {:032b} ({})", self.cpu.cpsr.get(), self.cpu.cpsr.get())).monospace()
            });

            ui.separator();

            for i in 0..4 {
                CollapsingHeader::new(format!("DMA Channel {}", i))
                    .default_open(if i == 1 || i == 2 { false } else { true })
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "{:08x} -> {:08x}, {:04x} bytes",
                                    self.cpu.dma.get().channels[i].src.value(),
                                    self.cpu.dma.get().channels[i].dst.value(),
                                    self.cpu.dma.get().channels[i].transfer_units()
                                ))
                                .monospace(),
                            );
                            ui.checkbox(
                                &mut self.cpu.dma.get().channels[i].is_enabled(),
                                RichText::new("Enabled").monospace(),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Trigger: {:?}, Repeat: {}",
                                    self.cpu.dma.get().channels[i].trigger(),
                                    self.cpu.dma.get().channels[i].is_repeat()
                                ))
                                .monospace(),
                            );
                        });
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!(
                                    "Src Ctrl: {:?}, Dst Ctrl: {:?}",
                                    self.cpu.dma.get().channels[i].src_addr_control(),
                                    self.cpu.dma.get().channels[i].dst_addr_control()
                                ))
                                .monospace(),
                            );
                        });
                    });
            }

            ui.separator();

            for i in 0..4 {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "TIMER {}: {:08x} ({:08x})",
                            i,
                            self.cpu.timers.get().timers[i].counter.value(),
                            self.cpu.timers.get().timers[i].reload.value(),
                        ))
                        .monospace(),
                    );
                    ui.checkbox(
                        &mut self.cpu.timers.get().timers[i]
                            .control
                            .value()
                            .contains(TimerControl::ENABLE),
                        RichText::new("Enabled").monospace(),
                    );
                });
            }
        });
    }
}
