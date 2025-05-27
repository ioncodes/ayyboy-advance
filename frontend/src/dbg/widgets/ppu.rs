use crate::event::RequestEvent;
use crossbeam_channel::Sender;
use egui::{CollapsingHeader, Color32, ColorImage, Context, RichText, TextureHandle, TextureOptions, Window};
use gba_core::video::{Frame, Rgb, PALETTE_TOTAL_ENTRIES, SCREEN_HEIGHT, SCREEN_WIDTH};

pub struct PpuWidget {
    pub frames: Box<[Frame; 6]>,
    pub palette: Box<[Rgb; PALETTE_TOTAL_ENTRIES]>,
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
            palette: Box::new([(0, 0, 0); PALETTE_TOTAL_ENTRIES]),
            bgmode3_frame0_texture: None,
            bgmode3_frame1_texture: None,
            bgmode4_frame0_texture: None,
            bgmode4_frame1_texture: None,
            bgmode5_frame0_texture: None,
            bgmode5_frame1_texture: None,
            event_tx: tx,
        }
    }

    pub fn update(&mut self, frames: Box<[Frame; 6]>, palette: Box<[Rgb; PALETTE_TOTAL_ENTRIES]>) {
        self.frames = frames;
        self.palette = palette;

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

        let _ = self.event_tx.send(RequestEvent::UpdatePpu);
    }

    pub fn render(&mut self, ctx: &Context) {
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
            CollapsingHeader::new("Background Modes")
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
    }
}
