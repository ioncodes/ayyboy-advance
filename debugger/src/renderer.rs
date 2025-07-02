use super::dbg::debugger::Debugger;
use super::event::ResponseEvent;
use crate::event::RequestEvent;
use chrono::Utc;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{vec2, CentralPanel, Color32, ColorImage, Context, Image, TextureHandle, TextureOptions};
use eframe::{App, CreationContext};
use egui::{Align2, Key, RichText, Window};
use egui_extras::{Column, TableBuilder};
use egui_toast::{Toast, ToastKind, ToastOptions, Toasts};
use gba_core::input::registers::KeyInput;
use gba_core::video::{Frame, Pixel, SCREEN_HEIGHT, SCREEN_WIDTH};
use image::{imageops, ImageBuffer, Rgb, RgbImage};

// TODO: make it a bit smaller for when im on my macbook
#[cfg(target_os = "macos")]
pub const SCALE: usize = 6;

#[cfg(not(target_os = "macos"))]
pub const SCALE: usize = 8;

pub struct Renderer {
    screen_texture: TextureHandle,
    screen_buffer: Frame,
    debugger: Debugger,
    display_rx: Receiver<Frame>,
    backend_tx: Sender<RequestEvent>,
    toasts: Toasts,
    running: bool,
}

impl Renderer {
    pub fn new(
        cc: &CreationContext, display_rx: Receiver<Frame>, backend_tx: Sender<RequestEvent>,
        backend_rx: Receiver<ResponseEvent>,
    ) -> Renderer {
        // TODO: debugger is currently designed for big screens
        // so scale everything down a bit in case im on my macbook
        #[cfg(target_os = "macos")]
        cc.egui_ctx.set_pixels_per_point(0.75);

        let screen_texture = cc.egui_ctx.load_texture(
            "screen_texture",
            ColorImage::new([SCREEN_WIDTH, SCREEN_HEIGHT], Color32::BLACK),
            TextureOptions::NEAREST,
        );
        let debugger = Debugger::new(
            backend_tx.clone(),
            backend_tx.clone(),
            backend_tx.clone(),
            backend_tx.clone(),
            backend_rx.clone(),
        );

        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        cc.egui_ctx.set_fonts(fonts);

        let toasts = Toasts::new();

        Renderer {
            screen_texture,
            screen_buffer: [[Pixel::Transparent; SCREEN_WIDTH]; SCREEN_HEIGHT],
            debugger,
            display_rx,
            backend_tx,
            toasts,
            running: false,
        }
    }

    pub fn update_screen(&mut self, texture: &Frame) {
        self.screen_buffer = texture.clone();

        let mut pixels = vec![Color32::BLACK; SCREEN_WIDTH * SCREEN_HEIGHT];

        for y in 0..SCREEN_HEIGHT {
            for x in 0..SCREEN_WIDTH {
                let color = texture[y][x];
                if let Pixel::Rgb(r, g, b) = color {
                    pixels[y * SCREEN_WIDTH + x] = Color32::from_rgba_premultiplied(r, g, b, 255);
                }
            }
        }

        let image = ColorImage {
            size: [SCREEN_WIDTH, SCREEN_HEIGHT],
            pixels,
        };

        self.screen_texture.set(image, TextureOptions::NEAREST);
    }

    pub fn handle_input(&mut self, ctx: &Context) {
        ctx.input(|i| {
            // Toggle debugger window
            if i.key_pressed(Key::F1) {
                self.debugger.toggle_window();
                self.running = false;
            }

            // Take a screenshot
            if i.key_pressed(Key::F2) {
                let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
                let screenshot_path = format!("screenshot_{}.png", timestamp);

                let img: RgbImage = ImageBuffer::from_fn(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32, |x, y| match self
                    .screen_buffer[y as usize][x as usize]
                {
                    Pixel::Transparent => Rgb([0, 0, 0]),
                    Pixel::Rgb(r, g, b) => Rgb([r, g, b]),
                });

                let scaled_img = imageops::resize(
                    &img,
                    (SCREEN_WIDTH * SCALE) as u32,
                    (SCREEN_HEIGHT * SCALE) as u32,
                    imageops::FilterType::Nearest,
                );

                scaled_img.save(&screenshot_path).unwrap();

                self.toasts.add(Toast {
                    text: format!("Screenshot saved as {}", screenshot_path).into(),
                    kind: ToastKind::Info,
                    options: ToastOptions::default().duration_in_seconds(3.0),
                    ..Default::default()
                });
            }

            // Run the emulator
            if i.key_pressed(Key::Space) && !self.running {
                self.backend_tx.send(RequestEvent::Run).unwrap();
                self.running = true;
            }

            // Update key state
            let mut key_state: Vec<(KeyInput, bool)> = Vec::new();
            key_state.push((KeyInput::A, i.key_down(Key::A)));
            key_state.push((KeyInput::B, i.key_down(Key::S)));
            key_state.push((KeyInput::START, i.key_down(Key::Enter)));
            key_state.push((KeyInput::SELECT, i.key_down(Key::Backspace)));
            key_state.push((KeyInput::UP, i.key_down(Key::ArrowUp)));
            key_state.push((KeyInput::DOWN, i.key_down(Key::ArrowDown)));
            key_state.push((KeyInput::LEFT, i.key_down(Key::ArrowLeft)));
            key_state.push((KeyInput::RIGHT, i.key_down(Key::ArrowRight)));
            key_state.push((KeyInput::L, i.key_down(Key::Q)));
            key_state.push((KeyInput::R, i.key_down(Key::W)));
            self.backend_tx.send(RequestEvent::UpdateKeyState(key_state)).unwrap();
        })
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

        if self.debugger.open {
            Window::new("Screen")
                .resizable(false)
                .show(ctx, |ui| ui.image(&self.screen_texture));
        }

        if !self.running && !self.debugger.open {
            Window::new("Controls")
                .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    TableBuilder::new(ui)
                        .columns(Column::auto(), 2)
                        .header(0.0, |mut header| {
                            header.col(|ui| {
                                ui.label(RichText::new("Key").italics());
                            });
                            header.col(|ui| {
                                ui.label(RichText::new("Action").italics());
                            });
                        })
                        .body(|mut body| {
                            body.row(0.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(RichText::new("F1").strong());
                                });
                                row.col(|ui| {
                                    ui.label("Toggle debugger window");
                                });
                            });

                            body.row(0.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(RichText::new("F2").strong());
                                });
                                row.col(|ui| {
                                    ui.label("Take a screenshot");
                                });
                            });

                            body.row(0.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(RichText::new("Space").strong());
                                });
                                row.col(|ui| {
                                    ui.label("Run the emulator");
                                });
                            });

                            body.row(0.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(RichText::new("A, S").strong());
                                });
                                row.col(|ui| {
                                    ui.label("A, B");
                                });
                            });

                            body.row(0.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(RichText::new("Q, W").strong());
                                });
                                row.col(|ui| {
                                    ui.label("L, R");
                                });
                            });

                            body.row(0.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(RichText::new("Enter, Backspace").strong());
                                });
                                row.col(|ui| {
                                    ui.label("Start, Select");
                                });
                            });

                            body.row(0.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(RichText::new("Arrow keys").strong());
                                });
                                row.col(|ui| {
                                    ui.label("D-pad");
                                });
                            });
                        });
                });
        }

        self.toasts.show(ctx);

        ctx.request_repaint();
    }
}
