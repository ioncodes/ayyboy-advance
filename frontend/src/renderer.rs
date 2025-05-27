use super::dbg::debugger::Debugger;
use super::event::ResponseEvent;
use crate::event::RequestEvent;
use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{vec2, CentralPanel, Color32, ColorImage, Context, Image, TextureHandle, TextureOptions};
use eframe::{App, CreationContext};
use egui::{Align2, Key, RichText, Window};
use egui_extras::{Column, TableBuilder};
use gba_core::input::registers::KeyInput;
use gba_core::video::{Frame, SCREEN_HEIGHT, SCREEN_WIDTH};

pub const SCALE: usize = 8;

pub struct Renderer {
    screen_texture: TextureHandle,
    debugger: Debugger,
    display_rx: Receiver<Frame>,
    backend_tx: Sender<RequestEvent>,
    running: bool,
}

impl Renderer {
    pub fn new(
        cc: &CreationContext, display_rx: Receiver<Frame>, backend_tx: Sender<RequestEvent>,
        backend_rx: Receiver<ResponseEvent>,
    ) -> Renderer {
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

        Renderer {
            screen_texture,
            debugger,
            display_rx,
            backend_tx,
            running: false,
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
        ctx.input(|i| {
            // Toggle debugger window
            if i.key_pressed(Key::F1) {
                self.debugger.toggle_window();
                self.running = false;
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
                                    ui.label(RichText::new("Space").strong());
                                });
                                row.col(|ui| {
                                    ui.label("Run the emulator");
                                });
                            });

                            body.row(0.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(RichText::new("A, S buttons").strong());
                                });
                                row.col(|ui| {
                                    ui.label("A, B buttons");
                                });
                            });

                            body.row(0.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(RichText::new("Q, W buttons").strong());
                                });
                                row.col(|ui| {
                                    ui.label("L, R buttons");
                                });
                            });

                            body.row(0.0, |mut row| {
                                row.col(|ui| {
                                    ui.label(RichText::new("Enter, Backspace buttons").strong());
                                });
                                row.col(|ui| {
                                    ui.label("Start, Select buttons");
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

        ctx.request_repaint();
    }
}
