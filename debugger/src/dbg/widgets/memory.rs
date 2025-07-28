use crate::dbg::tracked_value::TrackedValue;
use crate::dbg::widgets::DIRTY_COLOR;
use crate::event::RequestEvent;
use crossbeam_channel::Sender;
use egui::{ComboBox, RichText, ScrollArea, TextStyle};

const BYTES_PER_ROW: usize = 16;

pub struct MemoryWidget {
    memory_view: MemoryView,
    event_tx: Sender<RequestEvent>,
    memory: Vec<TrackedValue<u8>>,
}

impl MemoryWidget {
    pub fn new(tx: Sender<RequestEvent>) -> Self {
        let _ = tx.send(RequestEvent::UpdateMemory);
        Self {
            memory_view: MemoryView::Bios,
            event_tx: tx,
            memory: vec![TrackedValue::default(); 0x0FFF_FFFF + 1],
        }
    }

    pub fn update(&mut self, memory: Box<[u8; 0x0FFF_FFFF + 1]>) {
        memory.iter().enumerate().for_each(|(i, v)| self.memory[i].set(*v));
    }

    pub fn render_content(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::left_to_right(egui::Align::TOP), |ui| {
                ComboBox::from_label("Memory Map")
                    .selected_text(format!("{}", self.memory_view))
                    .show_ui(ui, |ui| {
                        for region in [
                            MemoryView::Bios,
                            MemoryView::OnboardWram,
                            MemoryView::OnchipWram,
                            MemoryView::IoRegisters,
                            MemoryView::PaletteRam,
                            MemoryView::Vram,
                            MemoryView::Oam,
                            MemoryView::GamePak,
                            MemoryView::GamePakSram,
                            MemoryView::Eeprom,
                        ] {
                            ui.selectable_value(&mut self.memory_view, region, region.to_string());
                        }
                    });
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

        ui.horizontal(|ui| {
            ui.label(RichText::new("        ").monospace().strong());
            for idx in 0..BYTES_PER_ROW {
                ui.label(RichText::new(format!("{:02X}", idx)).monospace().strong());
            }
            ui.add_space(5.0);
            ui.label(RichText::new("ASCII").monospace().strong());
        });

        let start = self.memory_view.start() as usize;
        let size = self.memory_view.size();
        debug_assert!(start + size <= self.memory.len());

        let mem_slice = &self.memory[start..start + size];
        let total_rows = (mem_slice.len() + BYTES_PER_ROW - 1) / BYTES_PER_ROW;

        ScrollArea::vertical().auto_shrink([false; 2]).show_rows(
            ui,
            ui.text_style_height(&TextStyle::Monospace),
            total_rows,
            |ui, rows| {
                for row in rows {
                    let base_addr = start + row * BYTES_PER_ROW;
                    let slice_off = row * BYTES_PER_ROW;
                    let take = BYTES_PER_ROW.min(mem_slice.len() - slice_off);
                    let chunk = &mem_slice[slice_off..slice_off + take];

                    ui.horizontal(|ui| {
                        ui.label(RichText::new(format!("{:08X}", base_addr)).monospace().strong());

                        for cell in chunk.iter() {
                            let mut richtext = RichText::new(format!("{:02X}", cell.get())).monospace();
                            if cell.has_changed() {
                                richtext = richtext.color(DIRTY_COLOR);
                            }
                            ui.label(richtext);
                        }

                        for _ in 0..(BYTES_PER_ROW - take) {
                            ui.monospace("");
                        }

                        ui.add_space(5.0);

                        let ascii: String = chunk
                            .iter()
                            .map(|b| {
                                let v: u8 = b.get();
                                if (0x20..=0x7E).contains(&v) { v as char } else { '.' }
                            })
                            .collect();
                        ui.monospace(ascii);
                    });
                }
            },
        );
    }
}

#[derive(PartialEq, Eq, Hash, Copy, Clone)]
pub enum MemoryView {
    Bios,
    OnboardWram,
    OnchipWram,
    IoRegisters,
    PaletteRam,
    Vram,
    Oam,
    GamePak,
    GamePakSram,
    Eeprom,
}

impl std::fmt::Display for MemoryView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryView::Bios => write!(f, "BIOS"),
            MemoryView::OnboardWram => write!(f, "On-board WRAM"),
            MemoryView::OnchipWram => write!(f, "On-chip WRAM"),
            MemoryView::IoRegisters => write!(f, "I/O Registers"),
            MemoryView::PaletteRam => write!(f, "Palette RAM"),
            MemoryView::Vram => write!(f, "VRAM"),
            MemoryView::Oam => write!(f, "OAM - OBJ Attributes"),
            MemoryView::GamePak => write!(f, "GamePak"),
            MemoryView::GamePakSram => write!(f, "GamePak SRAM"),
            MemoryView::Eeprom => write!(f, "EEPROM"),
        }
    }
}

impl MemoryView {
    pub fn range(self) -> std::ops::RangeInclusive<u32> {
        match self {
            MemoryView::Bios => 0x0000_0000..=0x0000_3FFF,
            MemoryView::OnboardWram => 0x0200_0000..=0x0203_FFFF,
            MemoryView::OnchipWram => 0x0300_0000..=0x0300_7FFF,
            MemoryView::IoRegisters => 0x0400_0000..=0x0400_03FE,
            MemoryView::PaletteRam => 0x0500_0000..=0x0500_03FF,
            MemoryView::Vram => 0x0600_0000..=0x0601_7FFF,
            MemoryView::Oam => 0x0700_0000..=0x0700_03FF,
            MemoryView::GamePak => 0x0800_0000..=0x09FF_FFFF,
            MemoryView::GamePakSram => 0x0E00_0000..=0x0E00_FFFF,
            MemoryView::Eeprom => 0x0D00_0000..=0x0DFF_FFFF,
        }
    }

    #[inline]
    pub fn size(self) -> usize {
        (*self.range().end() as usize) - (*self.range().start() as usize) + 1
    }

    #[inline]
    pub fn start(self) -> u32 {
        *self.range().start()
    }
}
