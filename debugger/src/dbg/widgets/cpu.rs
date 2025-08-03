use crate::dbg::tracked_value::TrackedValue;
use crate::dbg::widgets::DIRTY_COLOR;
use crate::event::RequestEvent;
use crossbeam_channel::Sender;
use egui::{CollapsingHeader, ComboBox, RichText, TextEdit};
use gba_core::arm7tdmi::registers::Psr;
use gba_core::arm7tdmi::timer::Timers;
use gba_core::memory::dma::Dma;
use gba_core::memory::registers::{Interrupt, TimerControl};

#[derive(Default, Copy, Clone)]
pub struct TrackedCpu {
    registers: [TrackedValue<u32>; 16],
    cpsr: TrackedValue<Psr>,
    dma: TrackedValue<Dma>,
    timers: TrackedValue<Timers>,
    ime: TrackedValue<bool>,
    ie: TrackedValue<Interrupt>,
    if_reg: TrackedValue<Interrupt>,
}

pub struct Cpu {
    pub registers: [u32; 16],
    pub cpsr: Psr,
    pub dma: Dma,
    pub timers: Timers,
    pub ime: bool,
    pub ie: Interrupt,
    pub if_reg: Interrupt,
}

pub struct CpuWidget {
    pub cpu: TrackedCpu,
    event_tx: Sender<RequestEvent>,
    breakpoint: String,
    selected_breakpoint: String,
    breakpoints: Vec<String>,
    should_auto_update: bool,
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
            should_auto_update: false,
        }
    }

    pub fn request_initial_update(&mut self) {
        let _ = self.event_tx.send(RequestEvent::UpdateCpu);
    }

    pub fn update(&mut self, cpu: Cpu) {
        self.cpu.registers.iter_mut().enumerate().for_each(|(i, reg)| {
            reg.set(cpu.registers[i]);
        });
        self.cpu.cpsr.set(cpu.cpsr);
        self.cpu.dma.set(cpu.dma);
        self.cpu.timers.set(cpu.timers);
        self.cpu.ime.set(cpu.ime);
        self.cpu.ie.set(cpu.ie);
        self.cpu.if_reg.set(cpu.if_reg);

        // Request another CPU update only if auto-updating is enabled
        if self.should_auto_update {
            let _ = self.event_tx.send(RequestEvent::UpdateCpu);
        }
    }

    pub fn render_content(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                if ui.button(format!("{} Run", egui_phosphor::regular::PLAY)).clicked() {
                    let _ = self.event_tx.send(RequestEvent::Run);
                    let _ = self.event_tx.send(RequestEvent::UpdateCpu); // Start the update chain
                    self.should_auto_update = true;
                }

                if ui.button(format!("{} Step", egui_phosphor::regular::STEPS)).clicked() {
                    let _ = self.event_tx.send(RequestEvent::Step);
                    let _ = self.event_tx.send(RequestEvent::UpdateCpu);
                    self.should_auto_update = false;
                }

                if ui.button(format!("{} Break", egui_phosphor::regular::PAUSE)).clicked() {
                    let _ = self.event_tx.send(RequestEvent::Break);
                    let _ = self.event_tx.send(RequestEvent::UpdateCpu);
                    self.should_auto_update = false;
                }
            });

            // Remove refresh button - it's now obsolete since updates are automatic
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
                RichText::new(format!("{}R{}: {:08X}", alignment, idx, reg.get()))
                    .monospace()
                    .color(DIRTY_COLOR)
            } else {
                RichText::new(format!("{}R{}: {:08X}", alignment, idx, reg.get())).monospace()
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
                .color(DIRTY_COLOR)
        } else {
            RichText::new(format!("CPSR: {:032b} ({})", self.cpu.cpsr.get(), self.cpu.cpsr.get())).monospace()
        });

        ui.separator();

        // Interrupt section
        CollapsingHeader::new("Interrupt Status")
            .default_open(true)
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let ime_text = if self.cpu.ime.has_changed() {
                        RichText::new(format!(
                            "IME: {}",
                            if self.cpu.ime.get() { "Enabled" } else { "Disabled" }
                        ))
                        .monospace()
                        .color(DIRTY_COLOR)
                    } else {
                        RichText::new(format!(
                            "IME: {}",
                            if self.cpu.ime.get() { "Enabled" } else { "Disabled" }
                        ))
                        .monospace()
                    };
                    ui.label(ime_text);
                });

                ui.horizontal(|ui| {
                    let ie_text = if self.cpu.ie.has_changed() {
                        RichText::new(format!(
                            "IE: {:016b} ({:04X})",
                            self.cpu.ie.get().bits(),
                            self.cpu.ie.get().bits()
                        ))
                        .monospace()
                        .color(DIRTY_COLOR)
                    } else {
                        RichText::new(format!(
                            "IE: {:016b} ({:04X})",
                            self.cpu.ie.get().bits(),
                            self.cpu.ie.get().bits()
                        ))
                        .monospace()
                    };
                    ui.label(ie_text);
                });

                ui.horizontal(|ui| {
                    let if_text = if self.cpu.if_reg.has_changed() {
                        RichText::new(format!(
                            "IF: {:016b} ({:04X})",
                            self.cpu.if_reg.get().bits(),
                            self.cpu.if_reg.get().bits()
                        ))
                        .monospace()
                        .color(DIRTY_COLOR)
                    } else {
                        RichText::new(format!(
                            "IF: {:016b} ({:04X})",
                            self.cpu.if_reg.get().bits(),
                            self.cpu.if_reg.get().bits()
                        ))
                        .monospace()
                    };
                    ui.label(if_text);
                });

                // Show individual interrupt flags with colored labels in organized layout
                let display_interrupt = |ui: &mut egui::Ui, name: &str, flag: Interrupt| {
                    let enabled = self.cpu.ie.get().contains(flag);
                    let pending = self.cpu.if_reg.get().contains(flag);

                    let color = if enabled && pending {
                        egui::Color32::from_rgb(255, 182, 193) // Light pink - both enabled and pending
                    } else if enabled {
                        egui::Color32::from_rgb(144, 238, 144) // Light green - enabled but not pending
                    } else if pending {
                        egui::Color32::from_rgb(255, 255, 182) // Light yellow - pending but not enabled
                    } else {
                        ui.visuals().text_color() // Default text color - neither
                    };

                    ui.colored_label(color, RichText::new(name).monospace());
                };

                // VBLANK HBLANK VCOUNT
                ui.horizontal(|ui| {
                    display_interrupt(ui, "VBLANK", Interrupt::VBLANK);
                    display_interrupt(ui, "HBLANK", Interrupt::HBLANK);
                    display_interrupt(ui, "VCOUNT", Interrupt::VCOUNT);
                });

                // TIMER0-3
                ui.horizontal(|ui| {
                    display_interrupt(ui, "TIMER0", Interrupt::TIMER0);
                    display_interrupt(ui, "TIMER1", Interrupt::TIMER1);
                    display_interrupt(ui, "TIMER2", Interrupt::TIMER2);
                    display_interrupt(ui, "TIMER3", Interrupt::TIMER3);
                });

                // DMA0-3
                ui.horizontal(|ui| {
                    display_interrupt(ui, "DMA0", Interrupt::DMA0);
                    display_interrupt(ui, "DMA1", Interrupt::DMA1);
                    display_interrupt(ui, "DMA2", Interrupt::DMA2);
                    display_interrupt(ui, "DMA3", Interrupt::DMA3);
                });

                // SERIAL KEYPAD GAMEPAK
                ui.horizontal(|ui| {
                    display_interrupt(ui, "SERIAL", Interrupt::SERIAL);
                    display_interrupt(ui, "KEYPAD", Interrupt::KEYPAD);
                    display_interrupt(ui, "GAMEPAK", Interrupt::GAMEPAK);
                });
            });

        ui.separator();

        CollapsingHeader::new("DMA Status").default_open(true).show(ui, |ui| {
            for i in 0..4 {
                let channel = &self.cpu.dma.get().channels[i];
                let enabled = channel.is_enabled();

                CollapsingHeader::new(format!("DMA Channel {}", i))
                    .default_open(i == 0 || i == 3)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let mut enabled_checkbox = enabled;
                            ui.add(egui::Checkbox::new(
                                &mut enabled_checkbox,
                                RichText::new("Enabled").monospace().color(ui.visuals().text_color()),
                            ));

                            let repeat = channel.is_repeat();
                            let mut repeat_checkbox = repeat;
                            ui.add(egui::Checkbox::new(
                                &mut repeat_checkbox,
                                RichText::new("Repeat").monospace().color(ui.visuals().text_color()),
                            ));

                            let irq = channel.trigger_irq();
                            let mut irq_checkbox = irq;
                            ui.add(egui::Checkbox::new(
                                &mut irq_checkbox,
                                RichText::new("IRQ").monospace().color(ui.visuals().text_color()),
                            ));
                        });

                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Source: {:08X}", channel.src.value())).monospace());
                            ui.label(RichText::new(format!("Dest: {:08X}", channel.dst.value())).monospace());
                        });

                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Units: {:04X}", channel.transfer_units())).monospace());
                            let transfer_size = if channel.transfer_size() == 4 {
                                "32-bit"
                            } else {
                                "16-bit"
                            };
                            ui.label(RichText::new(format!("Size: {}", transfer_size)).monospace());
                        });

                        ui.label(RichText::new(format!("Trigger: {:?}", channel.trigger())).monospace());

                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Src Ctrl: {:?}", channel.src_addr_control())).monospace());
                            ui.label(RichText::new(format!("Dst Ctrl: {:?}", channel.dst_addr_control())).monospace());
                        });
                    });
            }
        });

        ui.separator();

        CollapsingHeader::new("Timer Status").default_open(true).show(ui, |ui| {
            for i in 0..4 {
                let timer = &self.cpu.timers.get().timers[i];
                let control = timer.control.value();

                CollapsingHeader::new(format!("Timer {}", i))
                    .default_open(true)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("Counter: {:04X}", timer.counter.value())).monospace());
                            ui.label(RichText::new(format!("Reload: {:04X}", timer.reload.value())).monospace());
                        });

                        ui.horizontal(|ui| {
                            let enabled = control.contains(TimerControl::ENABLE);
                            let mut enabled_checkbox = enabled;
                            ui.add(egui::Checkbox::new(
                                &mut enabled_checkbox,
                                RichText::new("Enabled").monospace().color(ui.visuals().text_color()),
                            ));

                            let irq_enabled = control.contains(TimerControl::IRQ_ON_OVERFLOW);
                            let mut irq_checkbox = irq_enabled;
                            ui.add(egui::Checkbox::new(
                                &mut irq_checkbox,
                                RichText::new("IRQ").monospace().color(ui.visuals().text_color()),
                            ));
                        });
                    });
            }
        });
    }
}
