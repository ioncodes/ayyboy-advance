use super::dbg::debugger::Debugger;
use super::dbg::event::{RequestEvent, ResponseEvent};
use crate::video::{Frame, SCREEN_HEIGHT, SCREEN_WIDTH};
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{vec2, CentralPanel, Color32, ColorImage, Context, Image, TextureHandle, TextureOptions};
use eframe::{App, CreationContext};
use egui::Key;

pub const SCALE: usize = 4;

pub struct Renderer {
    screen_texture: TextureHandle,
    debugger: Debugger,
    display_rx: Receiver<Frame>,
}

impl Renderer {
    pub fn new(
        cc: &CreationContext, display_rx: Receiver<Frame>, debugger_tx: Sender<RequestEvent>,
        debugger_rx: Receiver<ResponseEvent>,
    ) -> Renderer {
        let screen_texture = cc.egui_ctx.load_texture(
            "screen_texture",
            ColorImage::new([SCREEN_WIDTH, SCREEN_HEIGHT], Color32::BLACK),
            TextureOptions::NEAREST,
        );
        let debugger = Debugger::new(
            debugger_tx.clone(),
            debugger_rx.clone(),
            debugger_tx.clone(),
            debugger_rx.clone(),
        );

        Renderer {
            screen_texture,
            debugger,
            display_rx,
        }
    }

    pub fn update_screen(&mut self, texture: &[[(u8, u8, u8); SCREEN_WIDTH]; SCREEN_HEIGHT]) {
        let mut pixels = vec![Color32::BLACK; SCREEN_WIDTH * SCREEN_HEIGHT];

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let color = texture[y][x];
                pixels[y * SCREEN_WIDTH + x] = Color32::from_rgba_premultiplied(color.0, color.1, color.2, 255);
            }
        }

        let image = ColorImage {
            size: [SCREEN_WIDTH, SCREEN_HEIGHT],
            pixels,
        };

        self.screen_texture.set(image, TextureOptions::NEAREST);
    }

    pub fn handle_input(&mut self, ctx: &Context) {
        if ctx.input(|i| i.key_pressed(Key::F1)) {
            self.debugger.toggle_window();
        }
    }
}

impl App for Renderer {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.handle_input(ctx);

        self.debugger.update(ctx);

        match self.display_rx.try_recv() {
            Ok(frame) => self.update_screen(&frame),
            _ => {}
        }

        CentralPanel::default().show(ctx, |ui| {
            let image = Image::new(&self.screen_texture);
            let image = image.fit_to_exact_size(vec2((SCREEN_WIDTH * SCALE) as f32, (SCREEN_HEIGHT * SCALE) as f32));
            image.paint_at(ui, ui.ctx().screen_rect());
        });

        ctx.request_repaint();
    }
}
