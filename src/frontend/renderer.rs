use eframe::egui::{
    vec2, CentralPanel, Color32, ColorImage, Context, Image, TextureHandle, TextureOptions,
};
use eframe::{App, CreationContext};
use tokio::sync::watch::Receiver;

use crate::video::{Frame, INTERNAL_HEIGHT, INTERNAL_WIDTH};

pub const SCALE: usize = 4;

pub struct Renderer {
    screen_texture: TextureHandle,
    rx: Receiver<Frame>,
}

impl Renderer {
    pub fn new(cc: &CreationContext, rx: Receiver<Frame>) -> Renderer {
        let screen_texture = cc.egui_ctx.load_texture(
            "screen_texture",
            ColorImage::new([INTERNAL_WIDTH, INTERNAL_HEIGHT], Color32::BLACK),
            TextureOptions::NEAREST,
        );

        Renderer { screen_texture, rx }
    }

    pub fn update_screen(&mut self, texture: &[[(u8, u8, u8); INTERNAL_WIDTH]; INTERNAL_HEIGHT]) {
        let mut pixels = vec![Color32::BLACK; INTERNAL_WIDTH * INTERNAL_HEIGHT];

        for y in 0..INTERNAL_WIDTH {
            for x in 0..INTERNAL_HEIGHT {
                let color = texture[y][x];
                pixels[y * INTERNAL_WIDTH + x] =
                    Color32::from_rgba_premultiplied(color.0, color.1, color.2, 255);
            }
        }

        let image = ColorImage {
            size: [INTERNAL_WIDTH, INTERNAL_HEIGHT],
            pixels,
        };

        self.screen_texture.set(image, TextureOptions::NEAREST);
    }
}

impl App for Renderer {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        match self.rx.has_changed() {
            Ok(true) => {
                let frame = self.rx.borrow_and_update().clone();
                self.update_screen(&frame);
            }
            _ => {}
        }

        CentralPanel::default().show(ctx, |ui| {
            let image = Image::new(&self.screen_texture);
            let image = image.fit_to_exact_size(vec2(
                (INTERNAL_WIDTH * SCALE) as f32,
                (INTERNAL_HEIGHT * SCALE) as f32,
            ));
            image.paint_at(ui, ui.ctx().screen_rect());
        });

        ctx.request_repaint();
    }
}
