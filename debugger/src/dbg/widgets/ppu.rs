use crate::dbg::widgets::TRANSPARENT_COLOR;
use crate::event::RequestEvent;
use crossbeam_channel::Sender;
use egui::{CollapsingHeader, Color32, ColorImage, Context, Image, RichText, TextureHandle, TextureOptions, Window};
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
    tilemap0_texture: Option<TextureHandle>,
    tilemap1_texture: Option<TextureHandle>,
    tilemap2_texture: Option<TextureHandle>,
    tilemap3_texture: Option<TextureHandle>,
    bgmode3_frame0_texture: Option<TextureHandle>,
    bgmode3_frame1_texture: Option<TextureHandle>,
    bgmode4_frame0_texture: Option<TextureHandle>,
    bgmode4_frame1_texture: Option<TextureHandle>,
    bgmode5_frame0_texture: Option<TextureHandle>,
    bgmode5_frame1_texture: Option<TextureHandle>,
    event_tx: Sender<RequestEvent>,
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
            tilemap0_texture: None,
            tilemap1_texture: None,
            tilemap2_texture: None,
            tilemap3_texture: None,
            bgmode3_frame0_texture: None,
            bgmode3_frame1_texture: None,
            bgmode4_frame0_texture: None,
            bgmode4_frame1_texture: None,
            bgmode5_frame0_texture: None,
            bgmode5_frame1_texture: None,
            event_tx: tx,
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

        update_tilemap_texture(&mut self.tilemap0_texture, self.tilemaps[0].0, &self.tilemaps[0].1);
        update_tilemap_texture(&mut self.tilemap1_texture, self.tilemaps[1].0, &self.tilemaps[1].1);
        update_tilemap_texture(&mut self.tilemap2_texture, self.tilemaps[2].0, &self.tilemaps[2].1);
        update_tilemap_texture(&mut self.tilemap3_texture, self.tilemaps[3].0, &self.tilemaps[3].1);

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

        if self.tilemap0_texture.is_none() {
            self.tilemap0_texture = Some(ctx.load_texture(
                "tilemap0",
                ColorImage::new([256, 256], Color32::BLACK),
                TextureOptions::default(),
            ));
        }
        if self.tilemap1_texture.is_none() {
            self.tilemap1_texture = Some(ctx.load_texture(
                "tilemap1",
                ColorImage::new([256, 256], Color32::BLACK),
                TextureOptions::default(),
            ));
        }
        if self.tilemap2_texture.is_none() {
            self.tilemap2_texture = Some(ctx.load_texture(
                "tilemap2",
                ColorImage::new([256, 256], Color32::BLACK),
                TextureOptions::default(),
            ));
        }
        if self.tilemap3_texture.is_none() {
            self.tilemap3_texture = Some(ctx.load_texture(
                "tilemap3",
                ColorImage::new([256, 256], Color32::BLACK),
                TextureOptions::default(),
            ));
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

    pub fn render(&mut self, ctx: &Context) {
        Window::new("PPU Registers").resizable(false).show(ctx, |ui| {
            CollapsingHeader::new("Display Control (DISP_CNT)")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("Background Mode: {}", self.registers.disp_cnt.bg_mode())).monospace(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "Frame Address: {:08X}",
                            self.registers.disp_cnt.frame_address()
                        ))
                        .monospace(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "OBJ Character Mapping: {}",
                            self.registers.disp_cnt.dimension()
                        ))
                        .monospace(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "BG 0 Enabled: {}",
                            self.registers.disp_cnt.contains(DispCnt::BG0_ON)
                        ))
                        .monospace(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "BG 1 Enabled: {}",
                            self.registers.disp_cnt.contains(DispCnt::BG1_ON)
                        ))
                        .monospace(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "BG 2 Enabled: {}",
                            self.registers.disp_cnt.contains(DispCnt::BG2_ON)
                        ))
                        .monospace(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "BG 3 Enabled: {}",
                            self.registers.disp_cnt.contains(DispCnt::BG3_ON)
                        ))
                        .monospace(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "OBJ Enabled: {}",
                            self.registers.disp_cnt.contains(DispCnt::OBJ_ON)
                        ))
                        .monospace(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "WIN 0 Enabled: {}",
                            self.registers.disp_cnt.contains(DispCnt::WIN0_ON)
                        ))
                        .monospace(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "WIN 1 Enabled: {}",
                            self.registers.disp_cnt.contains(DispCnt::WIN1_ON)
                        ))
                        .monospace(),
                    );
                });

            CollapsingHeader::new("Display Status (DISP_STAT)")
                .default_open(true)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(format!(
                            "VBLANK IRQ Enabled: {}",
                            self.registers.disp_stat.contains(DispStat::VBLANK_IRQ_ENABLE)
                        ))
                        .monospace(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "HBLANK IRQ Enabled: {}",
                            self.registers.disp_stat.contains(DispStat::HBLANK_IRQ_ENABLE)
                        ))
                        .monospace(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "VBLANK: {}",
                            self.registers.disp_stat.contains(DispStat::VBLANK_FLAG)
                        ))
                        .monospace(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "HBLANK: {}",
                            self.registers.disp_stat.contains(DispStat::HBLANK_FLAG)
                        ))
                        .monospace(),
                    );
                    ui.label(
                        RichText::new(format!(
                            "VCOUNT Enabled: {}",
                            self.registers.disp_stat.contains(DispStat::V_COUNTER_ENABLE)
                        ))
                        .monospace(),
                    );
                });

            CollapsingHeader::new("Background Control (BGxCNT)")
                .default_open(true)
                .show(ui, |ui| {
                    for (i, bg_cnt) in self.registers.bg_cnt.iter().enumerate() {
                        ui.label(
                            RichText::new(format!(
                                "BG{}CNT Screen Size: {}",
                                i,
                                bg_cnt.screen_size(i, self.registers.disp_cnt.bg_mode())
                            ))
                            .monospace(),
                        );
                        ui.label(
                            RichText::new(format!("BG{}CNT Tileset Address: {:08X}", i, bg_cnt.tileset_addr()))
                                .monospace(),
                        );
                        ui.label(
                            RichText::new(format!("BG{}CNT Tilemap Address: {:08X}", i, bg_cnt.tilemap_addr()))
                                .monospace(),
                        );
                        ui.label(RichText::new(format!("BG{}CNT Priority: {}", i, bg_cnt.priority())).monospace());
                        if i != 3 {
                            ui.separator();
                        }
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
        });

        Window::new("PPU Video").resizable(false).show(ctx, |ui| {
            CollapsingHeader::new("Tilemaps").default_open(true).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(texture) = &self.tilemap0_texture {
                        ui.image(texture);
                    }

                    if let Some(texture) = &self.tilemap1_texture {
                        ui.image(texture);
                    }

                    if let Some(texture) = &self.tilemap2_texture {
                        ui.image(texture);
                    }

                    if let Some(texture) = &self.tilemap3_texture {
                        ui.image(texture);
                    }
                });
            });

            CollapsingHeader::new("Palette").default_open(true).show(ui, |ui| {
                for (row_index, row) in self.palette.chunks(16).enumerate() {
                    ui.horizontal(|ui| {
                        for (col_index, color) in row.iter().enumerate() {
                            let i = row_index * 16 + col_index;
                            if let Pixel::Rgb(r, g, b) = color {
                                let color32 = Color32::from_rgb(*r, *g, *b);
                                ui.label(
                                    RichText::new(format!("{:04X}", i))
                                        .background_color(color32)
                                        .monospace(),
                                );
                            } else {
                                ui.label(RichText::new(format!("{:04X}", i)).monospace());
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
        });
    }
}
