use crate::dbg::widgets::TRANSPARENT_COLOR;
use crate::event::RequestEvent;
use crossbeam_channel::Sender;
use egui::{CollapsingHeader, Color32, ColorImage, Context, Image, RichText, TextureHandle, TextureOptions};
use gba_core::video::ppu::Sprite;
use gba_core::video::registers::{BgCnt, BgOffset, DispCnt, DispStat, InternalScreenSize, ObjSize};
use gba_core::video::{Frame, Pixel, SCREEN_HEIGHT, SCREEN_WIDTH};

#[derive(Default)]
pub struct PpuRegisters {
    pub disp_cnt: DispCnt,
    pub disp_stat: DispStat,
    pub bg_cnt: [BgCnt; 4],
    pub bg_vofs: [BgOffset; 4],
    pub bg_hofs: [BgOffset; 4],
}

pub struct PpuWidget {
    pub frames: Vec<Frame>,
    pub tilemaps: [(InternalScreenSize, Vec<Pixel>); 4],
    pub palette: Vec<Pixel>,
    pub registers: PpuRegisters,
    pub sprites: Vec<Sprite>,
    sprite_textures: Vec<Option<TextureHandle>>,
    tilemap_textures: [Option<TextureHandle>; 4],
    bgmode3_frame0_texture: Option<TextureHandle>,
    bgmode3_frame1_texture: Option<TextureHandle>,
    bgmode4_frame0_texture: Option<TextureHandle>,
    bgmode4_frame1_texture: Option<TextureHandle>,
    bgmode5_frame0_texture: Option<TextureHandle>,
    bgmode5_frame1_texture: Option<TextureHandle>,
    event_tx: Sender<RequestEvent>,
    selected_tilemap: usize,
    palette_scroll_offset: usize,
    show_all_tilemaps_popup: bool,
}

impl PpuWidget {
    pub fn new(tx: Sender<RequestEvent>) -> PpuWidget {
        let _ = tx.send(RequestEvent::UpdatePpu); // request initial PPU state

        PpuWidget {
            frames: Vec::new(),
            tilemaps: [
                (InternalScreenSize::Text256x256, Vec::new()),
                (InternalScreenSize::Text256x256, Vec::new()),
                (InternalScreenSize::Text256x256, Vec::new()),
                (InternalScreenSize::Text256x256, Vec::new()),
            ],
            palette: Vec::new(),
            registers: PpuRegisters::default(),
            sprites: Vec::new(),
            sprite_textures: vec![None; 128], // 128 sprites max
            tilemap_textures: [None, None, None, None],
            bgmode3_frame0_texture: None,
            bgmode3_frame1_texture: None,
            bgmode4_frame0_texture: None,
            bgmode4_frame1_texture: None,
            bgmode5_frame0_texture: None,
            bgmode5_frame1_texture: None,
            event_tx: tx,
            selected_tilemap: 0,
            palette_scroll_offset: 0,
            show_all_tilemaps_popup: false,
        }
    }

    pub fn update(
        &mut self, ctx: &Context, frames: Vec<Frame>, tilemaps: [(InternalScreenSize, Vec<Pixel>); 4],
        palette: Vec<Pixel>, registers: PpuRegisters, sprites: Vec<Sprite>,
    ) {
        self.frames = frames;
        self.tilemaps = tilemaps;
        self.palette = palette;
        self.registers = registers;
        self.sprites = sprites;

        let update_texture = |texture: &mut Option<TextureHandle>, frame: &Frame| {
            if let Some(texture) = texture {
                let mut pixels = vec![TRANSPARENT_COLOR; SCREEN_WIDTH * SCREEN_HEIGHT];
                for y in 0..SCREEN_HEIGHT {
                    for x in 0..SCREEN_WIDTH {
                        let color = frame[y][x];
                        if let Pixel::Rgb(r, g, b) = color {
                            pixels[y * SCREEN_WIDTH + x] = Color32::from_rgba_premultiplied(r, g, b, 255);
                        }
                    }
                }
                let image = ColorImage {
                    size: [SCREEN_WIDTH, SCREEN_HEIGHT],
                    pixels,
                };

                texture.set(image, TextureOptions::NEAREST);
            }
        };

        update_texture(&mut self.bgmode3_frame0_texture, &self.frames[0]);
        update_texture(&mut self.bgmode3_frame1_texture, &self.frames[1]);
        update_texture(&mut self.bgmode4_frame0_texture, &self.frames[2]);
        update_texture(&mut self.bgmode4_frame1_texture, &self.frames[3]);
        update_texture(&mut self.bgmode5_frame0_texture, &self.frames[4]);
        update_texture(&mut self.bgmode5_frame1_texture, &self.frames[5]);

        let update_tilemap_texture =
            |texture: &mut Option<TextureHandle>, size: InternalScreenSize, colors: &[Pixel]| {
                if let Some(texture) = texture {
                    let mut pixels = vec![TRANSPARENT_COLOR; colors.len()];
                    for (i, color) in colors.iter().enumerate() {
                        if let Pixel::Rgb(r, g, b) = color {
                            pixels[i] = Color32::from_rgba_premultiplied(*r, *g, *b, 255);
                        }
                    }

                    texture.set(
                        ColorImage {
                            size: [size.width(), size.height()],
                            pixels,
                        },
                        TextureOptions::NEAREST,
                    );
                }
            };

        for i in 0..4 {
            update_tilemap_texture(&mut self.tilemap_textures[i], self.tilemaps[i].0, &self.tilemaps[i].1);
        }

        let update_sprite_texture = |texture: &mut Option<TextureHandle>, sprite: &Sprite| {
            if let Some(texture) = texture {
                let pixels = sprite
                    .image
                    .iter()
                    .map(|&color| {
                        if let Pixel::Rgb(r, g, b) = color {
                            Color32::from_rgba_premultiplied(r, g, b, 255)
                        } else {
                            Color32::TRANSPARENT
                        }
                    })
                    .collect::<Vec<_>>();

                let size = match sprite.size {
                    ObjSize::Square8x8 => [8, 8],
                    ObjSize::Square16x16 => [16, 16],
                    ObjSize::Square32x32 => [32, 32],
                    ObjSize::Square64x64 => [64, 64],
                    ObjSize::Horizontal16x8 => [16, 8],
                    ObjSize::Horizontal32x8 => [32, 8],
                    ObjSize::Horizontal32x16 => [32, 16],
                    ObjSize::Horizontal64x32 => [64, 32],
                    ObjSize::Vertical8x16 => [8, 16],
                    ObjSize::Vertical8x32 => [8, 32],
                    ObjSize::Vertical16x32 => [16, 32],
                    ObjSize::Vertical32x64 => [32, 64],
                };

                texture.set(ColorImage { size: size, pixels }, TextureOptions::NEAREST);
            }
        };

        self.sprite_textures
            .iter_mut()
            .zip(self.sprites.iter())
            .for_each(|(texture, sprite)| {
                update_sprite_texture(texture, sprite);
            });

        for i in 0..4 {
            if self.tilemap_textures[i].is_none() {
                self.tilemap_textures[i] = Some(ctx.load_texture(
                    &format!("tilemap{}", i),
                    ColorImage::new([256, 256], Color32::BLACK),
                    TextureOptions::default(),
                ));
            }
        }
        if self.bgmode3_frame0_texture.is_none() {
            self.bgmode3_frame0_texture = Some(ctx.load_texture(
                "bgmode3_frame0",
                ColorImage::new([SCREEN_WIDTH, SCREEN_HEIGHT], Color32::BLACK),
                TextureOptions::default(),
            ));
        }
        if self.bgmode3_frame1_texture.is_none() {
            self.bgmode3_frame1_texture = Some(ctx.load_texture(
                "bgmode3_frame1",
                ColorImage::new([SCREEN_WIDTH, SCREEN_HEIGHT], Color32::BLACK),
                TextureOptions::default(),
            ));
        }
        if self.bgmode4_frame0_texture.is_none() {
            self.bgmode4_frame0_texture = Some(ctx.load_texture(
                "bgmode4_frame0",
                ColorImage::new([SCREEN_WIDTH, SCREEN_HEIGHT], Color32::BLACK),
                TextureOptions::default(),
            ));
        }
        if self.bgmode4_frame1_texture.is_none() {
            self.bgmode4_frame1_texture = Some(ctx.load_texture(
                "bgmode4_frame1",
                ColorImage::new([SCREEN_WIDTH, SCREEN_HEIGHT], Color32::BLACK),
                TextureOptions::default(),
            ));
        }
        if self.bgmode5_frame0_texture.is_none() {
            self.bgmode5_frame0_texture = Some(ctx.load_texture(
                "bgmode5_frame0",
                ColorImage::new([SCREEN_WIDTH, SCREEN_HEIGHT], Color32::BLACK),
                TextureOptions::default(),
            ));
        }
        if self.bgmode5_frame1_texture.is_none() {
            self.bgmode5_frame1_texture = Some(ctx.load_texture(
                "bgmode5_frame1",
                ColorImage::new([SCREEN_WIDTH, SCREEN_HEIGHT], Color32::BLACK),
                TextureOptions::default(),
            ));
        }
        self.sprite_textures.iter_mut().for_each(|texture| {
            if texture.is_none() {
                *texture = Some(ctx.load_texture(
                    "sprite",
                    ColorImage::new([8, 8], Color32::BLACK),
                    TextureOptions::default(),
                ));
            }
        });

        let _ = self.event_tx.send(RequestEvent::UpdatePpu);
    }

    pub fn render_registers_content(&mut self, ui: &mut egui::Ui) {
        CollapsingHeader::new("Display Control (DISP_CNT)")
            .default_open(true)
            .show(ui, |ui| {
                let bg_mode = self.registers.disp_cnt.bg_mode();
                ui.label(RichText::new(format!("Background Mode: {}", bg_mode)).monospace());

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "Frame Address: {:08X}",
                            self.registers.disp_cnt.frame_address()
                        ))
                        .monospace(),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!(
                            "OBJ Character Mapping: {}",
                            self.registers.disp_cnt.dimension()
                        ))
                        .monospace(),
                    );
                });

                // Background enable status with checkboxes
                ui.horizontal(|ui| {
                    for i in 0..4 {
                        let enabled = match i {
                            0 => self.registers.disp_cnt.contains(DispCnt::BG0_ON),
                            1 => self.registers.disp_cnt.contains(DispCnt::BG1_ON),
                            2 => self.registers.disp_cnt.contains(DispCnt::BG2_ON),
                            3 => self.registers.disp_cnt.contains(DispCnt::BG3_ON),
                            _ => false,
                        };
                        let mut enabled_checkbox = enabled;
                        ui.add(egui::Checkbox::new(
                            &mut enabled_checkbox,
                            RichText::new(format!("BG{}", i))
                                .monospace()
                                .color(ui.visuals().text_color()),
                        ));
                    }

                    let obj_enabled = self.registers.disp_cnt.contains(DispCnt::OBJ_ON);
                    let mut obj_checkbox = obj_enabled;
                    ui.add(egui::Checkbox::new(
                        &mut obj_checkbox,
                        RichText::new("OBJ").monospace().color(ui.visuals().text_color()),
                    ));
                });

                ui.horizontal(|ui| {
                    let win0_enabled = self.registers.disp_cnt.contains(DispCnt::WIN0_ON);
                    let mut win0_checkbox = win0_enabled;
                    ui.add(egui::Checkbox::new(
                        &mut win0_checkbox,
                        RichText::new("WIN0").monospace().color(ui.visuals().text_color()),
                    ));

                    let win1_enabled = self.registers.disp_cnt.contains(DispCnt::WIN1_ON);
                    let mut win1_checkbox = win1_enabled;
                    ui.add(egui::Checkbox::new(
                        &mut win1_checkbox,
                        RichText::new("WIN1").monospace().color(ui.visuals().text_color()),
                    ));
                });
            });

        CollapsingHeader::new("Display Status (DISP_STAT)")
            .default_open(true)
            .show(ui, |ui| {
                // Status flags with checkboxes
                ui.horizontal(|ui| {
                    let vblank = self.registers.disp_stat.contains(DispStat::VBLANK_FLAG);
                    let mut vblank_checkbox = vblank;
                    ui.add(egui::Checkbox::new(
                        &mut vblank_checkbox,
                        RichText::new("VBLANK").monospace().color(ui.visuals().text_color()),
                    ));

                    let hblank = self.registers.disp_stat.contains(DispStat::HBLANK_FLAG);
                    let mut hblank_checkbox = hblank;
                    ui.add(egui::Checkbox::new(
                        &mut hblank_checkbox,
                        RichText::new("HBLANK").monospace().color(ui.visuals().text_color()),
                    ));

                    let vcounter = self.registers.disp_stat.contains(DispStat::VCOUNTER_FLAG);
                    let mut vcounter_checkbox = vcounter;
                    ui.add(egui::Checkbox::new(
                        &mut vcounter_checkbox,
                        RichText::new("VCOUNTER").monospace().color(ui.visuals().text_color()),
                    ));
                });

                // IRQ enable flags with checkboxes
                ui.horizontal(|ui| {
                    let vblank_irq = self.registers.disp_stat.contains(DispStat::VBLANK_IRQ_ENABLE);
                    let mut vblank_irq_checkbox = vblank_irq;
                    ui.add(egui::Checkbox::new(
                        &mut vblank_irq_checkbox,
                        RichText::new("VBLANK IRQ").monospace().color(ui.visuals().text_color()),
                    ));

                    let hblank_irq = self.registers.disp_stat.contains(DispStat::HBLANK_IRQ_ENABLE);
                    let mut hblank_irq_checkbox = hblank_irq;
                    ui.add(egui::Checkbox::new(
                        &mut hblank_irq_checkbox,
                        RichText::new("HBLANK IRQ").monospace().color(ui.visuals().text_color()),
                    ));

                    let vcount_irq = self.registers.disp_stat.contains(DispStat::V_COUNTER_ENABLE);
                    let mut vcount_irq_checkbox = vcount_irq;
                    ui.add(egui::Checkbox::new(
                        &mut vcount_irq_checkbox,
                        RichText::new("VCOUNT IRQ").monospace().color(ui.visuals().text_color()),
                    ));
                });

                ui.horizontal(|ui| {
                    let vcount_setting = self.registers.disp_stat.vcount_setting();
                    ui.label(RichText::new(format!("VCOUNT Setting: {}", vcount_setting)).monospace());
                });
            });

        CollapsingHeader::new("Background Control (BGxCNT)")
            .default_open(true)
            .show(ui, |ui| {
                for (i, bg_cnt) in self.registers.bg_cnt.iter().enumerate() {
                    CollapsingHeader::new(format!("Background {} Control", i))
                        .default_open(true)
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("Priority: {}", bg_cnt.priority())).monospace());
                                ui.label(RichText::new(format!("Colors: {}", bg_cnt.bpp())).monospace());
                                let mosaic = bg_cnt.contains(BgCnt::MOSAIC);
                                let mut mosaic_checkbox = mosaic;
                                ui.add(egui::Checkbox::new(
                                    &mut mosaic_checkbox,
                                    RichText::new("Mosaic").monospace().color(ui.visuals().text_color()),
                                ));
                            });

                            ui.horizontal(|ui| {
                                ui.label(RichText::new(format!("Tileset: {:08X}", bg_cnt.tileset_addr())).monospace());
                                ui.label(RichText::new(format!("Tilemap: {:08X}", bg_cnt.tilemap_addr())).monospace());
                            });

                            ui.horizontal(|ui| {
                                let screen_size = bg_cnt.screen_size(i, self.registers.disp_cnt.bg_mode());
                                ui.label(RichText::new(format!("Size: {}", screen_size)).monospace());
                            });
                        });
                }
            });

        CollapsingHeader::new("Background Offsets (BGxVOFS/BGxHOFS)")
            .default_open(true)
            .show(ui, |ui| {
                for (i, (bg_vofs, bg_hofs)) in self
                    .registers
                    .bg_vofs
                    .iter()
                    .zip(self.registers.bg_hofs.iter())
                    .enumerate()
                {
                    ui.label(RichText::new(format!("BG{}VOFS: {}", i, bg_vofs.offset())).monospace());
                    ui.label(RichText::new(format!("BG{}HOFS: {}", i, bg_hofs.offset())).monospace());
                    if i != 3 {
                        ui.separator();
                    }
                }
            });
    }

    pub fn render_video_content(&mut self, ui: &mut egui::Ui) {
        CollapsingHeader::new("Tilemaps").default_open(true).show(ui, |ui| {
            ui.horizontal(|ui| {
                for i in 0..4 {
                    ui.selectable_value(&mut self.selected_tilemap, i, format!("Background {}", i));
                }

                if ui.button("View All").clicked() {
                    self.show_all_tilemaps_popup = true;
                }
            });

            if let Some(texture) = &self.tilemap_textures[self.selected_tilemap] {
                ui.add(
                    Image::from_texture(texture)
                        .fit_to_exact_size(egui::vec2(200.0, 200.0))
                        .texture_options(egui::TextureOptions::NEAREST),
                );
            }
        });

        CollapsingHeader::new("Palette").default_open(true).show(ui, |ui| {
            ui.horizontal(|ui| {
                let prev_enabled = self.palette_scroll_offset > 0;
                let next_enabled = self.palette_scroll_offset + 256 < self.palette.len();

                ui.add_enabled_ui(prev_enabled, |ui| {
                    if ui.button("◀ Page 1").clicked() {
                        self.palette_scroll_offset = 0;
                    }
                });

                let current_page = if self.palette_scroll_offset == 0 { 1 } else { 2 };
                ui.label(
                    RichText::new(format!(
                        "Page {} | Colors {:#04X}-{:#04X}",
                        current_page,
                        self.palette_scroll_offset,
                        (self.palette_scroll_offset + 255).min(self.palette.len().saturating_sub(1))
                    ))
                    .monospace(),
                );

                ui.add_enabled_ui(next_enabled, |ui| {
                    if ui.button("Page 2 ▶").clicked() {
                        self.palette_scroll_offset = 256;
                    }
                });
            });

            let end_idx = (self.palette_scroll_offset + 256).min(self.palette.len());
            let visible_palette = &self.palette[self.palette_scroll_offset..end_idx];

            for (row_index, row) in visible_palette.chunks(16).enumerate() {
                ui.horizontal(|ui| {
                    for (col_index, color) in row.iter().enumerate() {
                        let i = self.palette_scroll_offset + row_index * 16 + col_index;
                        if let Pixel::Rgb(r, g, b) = color {
                            let color32 = Color32::from_rgb(*r, *g, *b);
                            ui.add(
                                egui::widgets::Button::new(format!("{:02X}", i & 0xFF))
                                    .fill(color32)
                                    .min_size(egui::vec2(25.0, 20.0)),
                            )
                            .on_hover_text(format!("Index: {:#04X}, RGB: ({}, {}, {})", i, r, g, b));
                        }
                    }
                });
            }
        });

        CollapsingHeader::new("Sprites").default_open(true).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    for sprite in &self.sprites {
                        let texture = self.sprite_textures.get(sprite.id).and_then(|t| t.as_ref()).unwrap();
                        ui.add(
                            Image::from_texture(texture)
                                .fit_to_original_size(2.0)
                                .texture_options(egui::TextureOptions::NEAREST),
                        )
                        .on_hover_text(
                            RichText::new(format!(
                                "ID: {}, Tile Nr: {}\nX: {}, Y: {}\nSize: {}\nShape: {:?}\nPriority: {:?}\nPalette: {}\nColor Depth: {}\nFlip X: {}, Flip Y: {}\nAttribute 0: {:04X} @ {:08X}\nAttribute 1: {:04X} @ {:08X}\nAttribute 2: {:04X} @ {:08X}",
                                sprite.id,
                                sprite.tile_number,
                                sprite.x,
                                sprite.y,
                                sprite.size,
                                sprite.shape,
                                sprite.priority,
                                sprite.palette,
                                sprite.attr0.bpp(),
                                sprite.x_flip,
                                sprite.y_flip,
                                sprite.attr0.bits(),
                                sprite.attr0_addr,
                                sprite.attr1.bits(),
                                sprite.attr1_addr,
                                sprite.attr2.bits(),
                                sprite.attr2_addr,
                            ))
                            .monospace(),
                        );
                    }
                });
            });

        CollapsingHeader::new("Internal Frames")
            .default_open(false)
            .show(ui, |ui| {
                ui.label("Background Mode 3");
                ui.horizontal(|ui| {
                    if let Some(texture) = &self.bgmode3_frame0_texture {
                        ui.image(texture);
                    }
                    if let Some(texture) = &self.bgmode3_frame1_texture {
                        ui.image(texture);
                    }
                });

                ui.label("Background Mode 4");
                ui.horizontal(|ui| {
                    if let Some(texture) = &self.bgmode4_frame0_texture {
                        ui.image(texture);
                    }
                    if let Some(texture) = &self.bgmode4_frame1_texture {
                        ui.image(texture);
                    }
                });

                ui.label("Background Mode 5");
                ui.horizontal(|ui| {
                    if let Some(texture) = &self.bgmode5_frame0_texture {
                        ui.image(texture);
                    }
                    if let Some(texture) = &self.bgmode5_frame1_texture {
                        ui.image(texture);
                    }
                });
            });

        // Popup window for all tilemaps
        if self.show_all_tilemaps_popup {
            egui::Window::new("All Backgrounds")
                .resizable(true)
                .auto_sized()
                .open(&mut self.show_all_tilemaps_popup)
                .show(ui.ctx(), |ui| {
                    for i in 0..4 {
                        ui.heading(format!("Background {}", i));
                        if let Some(texture) = &self.tilemap_textures[i] {
                            ui.add(
                                Image::from_texture(texture)
                                    .fit_to_exact_size(egui::vec2(256.0, 256.0))
                                    .texture_options(egui::TextureOptions::NEAREST),
                            );
                        }
                    }
                });
        }
    }
}
