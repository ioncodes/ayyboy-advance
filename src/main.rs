#![feature(new_uninit)]

mod arm7tdmi;
mod frontend;
mod memory;
mod video;

use arm7tdmi::cpu::{Cpu, ProcessorMode};
use eframe::NativeOptions;
use egui::ViewportBuilder;
use frontend::renderer::{Renderer, SCALE};
use memory::mmio::Mmio;

use tokio::sync::watch::{self, Receiver, Sender};
use video::{Frame, SCREEN_HEIGHT, SCREEN_WIDTH};

const ARM_TEST: &[u8] = include_bytes!("../external/gba-tests/arm/arm.gba");
// const ARM_TEST: &[u8] = include_bytes!("../external/gba-div-test/out/rom.gba"); // just a div test
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/panda.gba"); // works
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/methharold.gba"); // works
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/gang.gba"); // works
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/gang-ldmstm.gba");
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/armfuck.gba");
const BIOS: &[u8] = include_bytes!("../external/gba_bios.bin");

fn main() {
    env_logger::builder().format_timestamp(None).init();

    let frame = [[(0u8, 0u8, 0u8); SCREEN_WIDTH]; SCREEN_HEIGHT];
    let (tx, rx): (Sender<Frame>, Receiver<Frame>) = watch::channel(frame);

    std::thread::spawn(move || {
        let mut mmio = Mmio::new();
        mmio.load(0x00000000, BIOS); // bios addr
        mmio.load(0x08000000, ARM_TEST); // gamepak addr

        let mut cpu = Cpu::new();
        cpu.registers.r[13] = 0x03007f00; // sp
        cpu.registers.r[15] = 0x08000000; // pc
        cpu.set_processor_mode(ProcessorMode::User);

        let mut frame_rendered = false;

        loop {
            cpu.tick(&mut mmio);
            mmio.tick_components();

            if mmio.ppu.scanline == 160 && !frame_rendered {
                tx.send(mmio.ppu.get_frame()).unwrap();
                frame_rendered = true;
            } else if mmio.ppu.scanline == 0 && frame_rendered {
                frame_rendered = false;
            }
        }
    });

    let native_options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([
                (SCREEN_WIDTH * SCALE) as f32,
                (SCREEN_HEIGHT * SCALE) as f32,
            ])
            .with_resizable(true),
        ..Default::default()
    };

    let _ = eframe::run_native(
        "ayyboy advance",
        native_options,
        Box::new(move |cc| Ok(Box::new(Renderer::new(cc, rx)))),
    );
}
