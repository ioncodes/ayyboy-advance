use crate::frontend::dbg::tracked_value::TrackedValue;
use crate::frontend::event::RequestEvent;
use crossbeam_channel::Sender;
use egui::{ComboBox, Context, RichText, Window};
use egui_extras::{Column, TableBuilder};

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

    pub fn size(self) -> usize {
        *self.range().end() as usize - *self.range().start() as usize + 1
    }

    pub fn start(self) -> u32 {
        *self.range().start()
    }
}

pub struct MemoryWidget {
    memory_view: MemoryView,
    event_tx: Sender<RequestEvent>,
    memory: Vec<TrackedValue<u8>>,
}

impl MemoryWidget {
    pub fn new(tx: Sender<RequestEvent>) -> MemoryWidget {
        let _ = tx.send(RequestEvent::UpdateMemory); // request initial memory state

        MemoryWidget {
            memory_view: MemoryView::Bios,
            event_tx: tx,
            memory: vec![TrackedValue::default(); 0x0FFFFFFF + 1],
        }
    }

    pub fn update(&mut self, memory: Box<[u8; 0x0FFFFFFF + 1]>) {
        memory[..].iter().enumerate().for_each(|(i, v)| {
            self.memory[i].set(*v);
        });
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

            ui.separator();

            TableBuilder::new(ui)
                .columns(Column::auto(), 17)
                .header(0.0, |mut header| {
                    header.col(|ui| {
                        ui.label("");
                    });
                    for idx in 0..16 {
                        header.col(|ui| {
                            ui.label(RichText::new(format!("{:02x}", idx)).monospace().strong());
                        });
                    }
                })
                .body(|body| {
                    let range_start = self.memory_view.start();
                    let range_count = self.memory_view.size();

                    body.rows(0.0, range_count / 16, |mut row| {
                        let idx = row.index() as u32;
                        let addr = range_start + (idx * 16);

                        row.col(|ui| {
                            ui.label(RichText::new(format!("0x{:08x}", addr)).monospace().strong());
                        });

                        for idx in 0..16 {
                            let addr = addr + idx;

                            row.col(|ui| {
                                let value = self.memory[addr as usize];
                                let mut richtext = RichText::new(format!("{:02x}", value.get())).monospace();
                                if value.has_changed() {
                                    richtext =
                                        richtext.color(egui::Color32::from_rgba_premultiplied(250, 160, 160, 255))
                                }
                                ui.label(richtext);
                            });
                        }
                    });
                });
        });
    }
}
