use crate::event::RequestEvent;
use crossbeam_channel::Sender;
use egui::{
    CollapsingHeader, Color32, ColorImage, Context, RichText, ScrollArea, TextureHandle, TextureOptions, Window,
};
use gba_core::video::registers::{BgCnt, DispCnt, DispStat, InternalScreenSize};
use gba_core::video::{Frame, Rgb, PALETTE_TOTAL_ENTRIES, SCREEN_HEIGHT, SCREEN_WIDTH};

#[derive(Default)]
pub struct PpuRegisters {
    pub disp_cnt: DispCnt,
    pub disp_stat: DispStat,
    pub bg_cnt: [BgCnt; 4],
}

pub struct PpuWidget {
    pub frames: Box<[Frame; 6]>,
    pub tileset: Vec<Rgb>,
    pub tilemaps: [(InternalScreenSize, Vec<Rgb>); 4],
    pub palette: Box<[Rgb; PALETTE_TOTAL_ENTRIES]>,
    pub registers: PpuRegisters,
    tileset_texture: Option<TextureHandle>,
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
            frames: Box::new([[[(0, 0, 0); SCREEN_WIDTH]; SCREEN_HEIGHT]; 6]),
            tileset: Vec::new(),
            tilemaps: [
                (InternalScreenSize::Size256x256, Vec::new()),
                (InternalScreenSize::Size512x512, Vec::new()),
                (InternalScreenSize::Size256x256, Vec::new()),
                (InternalScreenSize::Size512x512, Vec::new()),
            ],
            palette: Box::new([(0, 0, 0); PALETTE_TOTAL_ENTRIES]),
            registers: PpuRegisters::default(),
            tileset_texture: None,
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
        &mut self, frames: Box<[Frame; 6]>, tileset: Vec<Rgb>, tilemaps: [(InternalScreenSize, Vec<Rgb>); 4],
        palette: Box<[Rgb; PALETTE_TOTAL_ENTRIES]>, registers: PpuRegisters,
    ) {
        self.frames = frames;
        self.tileset = tileset;
        self.tilemaps = tilemaps;
        self.palette = palette;
        self.registers = registers;

        let update_texture = |texture: &mut Option<TextureHandle>, frame: &Frame| {
            if let Some(texture) = texture {
                let mut pixels = vec![Color32::BLACK; SCREEN_WIDTH * SCREEN_HEIGHT];
                for y in 0..SCREEN_HEIGHT {
                    for x in 0..SCREEN_WIDTH {
                        let color = frame[y][x];
                        pixels[y * SCREEN_WIDTH + x] = Color32::from_rgba_premultiplied(color.0, color.1, color.2, 255);
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

        if let Some(tileset_texture) = &mut self.tileset_texture {
            let mut pixels = vec![Color32::BLACK; self.tileset.len()];
            for (i, color) in self.tileset.iter().enumerate() {
                pixels[i] = Color32::from_rgb(color.0, color.1, color.2);
            }
            let image = ColorImage {
                size: [16 * 8, 64 * 8],
                pixels,
            };
            tileset_texture.set(image, TextureOptions::NEAREST);
        }

        let update_tilemap_texture = |texture: &mut Option<TextureHandle>, size: InternalScreenSize, colors: &[Rgb]| {
            if let Some(texture) = texture {
                let mut pixels = vec![Color32::BLACK; colors.len()];
                for (i, color) in colors.iter().enumerate() {
                    pixels[i] = Color32::from_rgb(color.0, color.1, color.2);
                }

                let dimensions = match size {
                    InternalScreenSize::Size256x256 => [256, 256],
                    InternalScreenSize::Size512x512 => [512, 512],
                    InternalScreenSize::Size256x512 => [256, 512],
                    InternalScreenSize::Size512x256 => [512, 256],
                };

                texture.set(
                    ColorImage {
                        size: dimensions,
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

        let _ = self.event_tx.send(RequestEvent::UpdatePpu);
    }

    pub fn render(&mut self, ctx: &Context) {
        if self.tileset_texture.is_none() {
            self.tileset_texture = Some(ctx.load_texture(
                "tileset",
                ColorImage::new([16 * 8, 64 * 8], Color32::BLACK),
                TextureOptions::default(),
            ));
        }
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

        Window::new("PPU").resizable(false).show(ctx, |ui| {
            CollapsingHeader::new("Registers").default_open(true).show(ui, |ui| {
                ui.label(RichText::new("Display Control (DISP_CNT):").monospace().strong());
                ui.label(RichText::new(format!("Background Mode: {}", self.registers.disp_cnt.bg_mode())).monospace());
                ui.label(
                    RichText::new(format!(
                        "Frame Address: {:08x}",
                        self.registers.disp_cnt.frame_address()
                    ))
                    .monospace(),
                );

                ui.separator();

                ui.label(RichText::new("Display Status (DISP_STAT):").monospace().strong());
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

                ui.separator();

                ui.label(RichText::new("Background Control (BGxCNT):").monospace().strong());
                for (i, bg_cnt) in self.registers.bg_cnt.iter().enumerate() {
                    ui.label(RichText::new(format!("BG{}CNT Screen Size: {}", i, bg_cnt.screen_size())).monospace());
                    ui.label(
                        RichText::new(format!("BG{}CNT Char Base Address: {:08x}", i, bg_cnt.tileset_addr()))
                            .monospace(),
                    );
                    ui.label(
                        RichText::new(format!("BG{}CNT Screen Base Address: {:08x}", i, bg_cnt.tilemap_addr()))
                            .monospace(),
                    );
                }
            });

            CollapsingHeader::new("Palette").default_open(true).show(ui, |ui| {
                for (row_index, row) in self.palette.chunks(16).enumerate() {
                    ui.horizontal(|ui| {
                        for (col_index, color) in row.iter().enumerate() {
                            let i = row_index * 16 + col_index;
                            let color32 = Color32::from_rgb(color.0, color.1, color.2);
                            ui.label(
                                RichText::new(format!("{:04X}", i))
                                    .background_color(color32)
                                    .monospace(),
                            );
                        }
                    });
                }
            });
        });

        Window::new("PPU Textures").resizable(false).show(ctx, |ui| {
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

            CollapsingHeader::new("Tileset").default_open(false).show(ui, |ui| {
                ScrollArea::vertical()
                    .auto_shrink([false, true])
                    .max_height(64.0 * 8.0 * 4.0)
                    .show(ui, |ui| {
                        if let Some(texture) = &self.tileset_texture {
                            ui.image(texture);
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
