use crate::frontend::dbg::event::{RequestEvent, ResponseEvent};
use crossbeam_channel::{Receiver, Sender};
use egui::{ComboBox, Context, RichText, ScrollArea, Window};

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub enum MemoryView {
    Bios,
    OnboardWram,
    OnchipWram,
    PaletteRam,
    Vram,
    Oam,
    GamePak,
    GamePakSram,
}

impl std::fmt::Display for MemoryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryView::Bios => write!(f, "BIOS"),
            MemoryView::OnboardWram => write!(f, "On-board WRAM"),
            MemoryView::OnchipWram => write!(f, "On-chip WRAM"),
            MemoryView::PaletteRam => write!(f, "Palette RAM"),
            MemoryView::Vram => write!(f, "VRAM"),
            MemoryView::Oam => write!(f, "OAM - OBJ Attributes"),
            MemoryView::GamePak => write!(f, "GamePak"),
            MemoryView::GamePakSram => write!(f, "GamePak SRAM"),
        }
    }
}

impl MemoryView {
    pub fn range(self) -> std::ops::RangeInclusive<u32> {
        match self {
            MemoryView::Bios => 0x00000000..=0x00003FFF,
            MemoryView::OnboardWram => 0x02000000..=0x0203FFFF,
            MemoryView::OnchipWram => 0x03000000..=0x03007FFF,
            MemoryView::PaletteRam => 0x05000000..=0x050003FF,
            MemoryView::Vram => 0x06000000..=0x06017FFF,
            MemoryView::Oam => 0x07000000..=0x070003FF,
            MemoryView::GamePak => 0x08000000..=0x09FFFFFF,
            MemoryView::GamePakSram => 0x0E000000..=0x0E00FFFF,
        }
    }
}

pub struct MemoryWidget {
    memory_view: MemoryView,
    event_rx: Receiver<ResponseEvent>,
    event_tx: Sender<RequestEvent>,
    memory: Box<[u8; 0x0FFFFFFF + 1]>,
}

impl MemoryWidget {
    pub fn new(rx: Receiver<ResponseEvent>, tx: Sender<RequestEvent>) -> MemoryWidget {
        MemoryWidget {
            memory_view: MemoryView::Bios,
            event_rx: rx,
            event_tx: tx,
            memory: unsafe {
                let memory = Box::<[u8; 0x0FFFFFFF + 1]>::new_zeroed();
                memory.assume_init()
            },
        }
    }

    pub fn update(&mut self) {
        match self.event_rx.try_recv() {
            Ok(ResponseEvent::Memory(memory)) => {
                self.memory = memory;
            }
            _ => {}
        }
    }

    pub fn render(&mut self, ctx: &Context) {
        Window::new("Memory").resizable(false).min_width(400.0).show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                    ComboBox::from_label("Memory Map")
                        .selected_text(format!("{}", self.memory_view))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut self.memory_view, MemoryView::Bios, "BIOS");
                            ui.selectable_value(&mut self.memory_view, MemoryView::OnboardWram, "On-board WRAM");
                            ui.selectable_value(&mut self.memory_view, MemoryView::OnchipWram, "On-chip WRAM");
                            ui.selectable_value(&mut self.memory_view, MemoryView::PaletteRam, "Palette RAM");
                            ui.selectable_value(&mut self.memory_view, MemoryView::Vram, "VRAM");
                            ui.selectable_value(&mut self.memory_view, MemoryView::Oam, "OAM - OBJ Attributes");
                            ui.selectable_value(&mut self.memory_view, MemoryView::GamePak, "GamePak");
                            ui.selectable_value(&mut self.memory_view, MemoryView::GamePakSram, "GamePak SRAM");
                        });
                    ui.add_space(3.0);
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                    if ui
                        .button(format!("{} Refresh", egui_phosphor::regular::ARROW_CLOCKWISE))
                        .clicked()
                    {
                        let _ = self.event_tx.send(RequestEvent::UpdateMemory);
                    }
                });
            });

            ui.add_space(3.0);
            ui.label(
                RichText::new("            00 01 02 03 04 05 06 07 08 09 0a 0b 0c 0d 0e 0f")
                    .monospace()
                    .strong(),
            );

            ScrollArea::vertical().show(ui, |ui| {
                ui.vertical(|ui| {
                    let range = self.memory_view.range();

                    range.step_by(16).for_each(|addr| {
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(format!("0x{:08x}", addr)).monospace().strong());

                            let mut line = String::new();
                            for offset in 0..16 {
                                let addr = addr + offset;
                                line += &format!(" {:02x}", self.memory[addr as usize]);
                            }

                            ui.label(RichText::new(line).monospace());
                        });
                    });
                });
            });
        });
    }
}
