#![feature(new_zeroed_alloc)]
#![feature(if_let_guard)]
#![feature(let_chains)]

mod arm7tdmi;
mod audio;
mod emulator;
mod frontend;
mod input;
mod memory;
mod script;
mod tests;
mod video;

use crate::emulator::Emulator;
use crate::frontend::renderer::SCALE;
use clap::Parser;
use crossbeam_channel::{bounded, Receiver, Sender};
use eframe::NativeOptions;
use log::LevelFilter;
use simple_logger::SimpleLogger;
use video::{Frame, SCREEN_HEIGHT, SCREEN_WIDTH};

#[derive(Parser, Debug)]
struct Args {
    /// Enable trace-level logging (highest verbosity, incl. cpu dump and mmio events)
    #[arg(long)]
    trace: bool,

    /// Enable debug-level logging (mostly just cpu instructions)
    #[arg(long)]
    debug: bool,

    /// Path to a custom script file
    #[arg(long)]
    script: Option<String>,

    /// Path to the ROM file
    #[arg(long)]
    rom: String,
}

fn main() {
    let args = Args::parse();

    SimpleLogger::new()
        .with_level(LevelFilter::Off)
        .with_module_level(
            module_path!(),
            if args.trace {
                LevelFilter::Trace
            } else if args.debug {
                LevelFilter::Debug
            } else {
                LevelFilter::Warn
            },
        )
        .init()
        .unwrap();

    let (display_tx, display_rx): (Sender<Frame>, Receiver<Frame>) = bounded(1);
    let (dbg_req_tx, dbg_req_rx) = bounded(25);
    let (dbg_resp_tx, dbg_resp_rx) = bounded(25);

    let mut emulator = Emulator::new(display_tx, dbg_req_rx, dbg_resp_tx, args.script, args.rom);

    std::thread::spawn(move || {
        emulator.run();
    });

    let native_options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([(SCREEN_WIDTH * SCALE) as f32, (SCREEN_HEIGHT * SCALE) as f32])
            .with_resizable(false),
        vsync: false,
        default_theme: eframe::Theme::Dark,
        ..Default::default()
    };

    let _ = eframe::run_native(
        "ayyboy advance",
        native_options,
        Box::new(move |cc| {
            Ok(Box::new(frontend::renderer::Renderer::new(
                cc,
                display_rx,
                dbg_req_tx,
                dbg_resp_rx,
            )))
        }),
    );

    // spdlog::default_logger().flush();
}
