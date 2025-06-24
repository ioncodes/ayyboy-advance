#![feature(new_zeroed_alloc)]
#![feature(if_let_guard)]

mod dbg;
mod emulator;
mod event;
mod renderer;

use crate::emulator::Emulator;
use crate::renderer::SCALE;
use clap::Parser;
use crossbeam_channel::{self, Receiver, Sender};
use eframe::NativeOptions;
use gba_core::video::{Frame, SCREEN_HEIGHT, SCREEN_WIDTH};
use log::LevelFilter;
use renderer::Renderer;
use shadow_rs::shadow;
use simple_logger::SimpleLogger;

shadow!(build_info);

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
            "gba_core",
            if args.trace {
                LevelFilter::Trace
            } else if args.debug {
                LevelFilter::Debug
            } else {
                LevelFilter::Info
            },
        )
        .init()
        .unwrap();

    let (display_tx, display_rx): (Sender<Frame>, Receiver<Frame>) = crossbeam_channel::bounded(1);
    let (dbg_req_tx, dbg_req_rx) = crossbeam_channel::bounded(25);
    let (dbg_resp_tx, dbg_resp_rx) = crossbeam_channel::bounded(25);

    let mut emulator = Emulator::new(display_tx, dbg_req_rx, dbg_resp_tx, args.script, args.rom);
    let rom_title = emulator.rom_title.clone();

    std::thread::spawn(move || {
        emulator.run();
    });

    let native_options = NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([(SCREEN_WIDTH * SCALE) as f32, (SCREEN_HEIGHT * SCALE) as f32])
            .with_resizable(false),
        vsync: false,
        ..Default::default()
    };

    let _ = eframe::run_native(
        &format!("ayyboy advance [{}] - {}", build_info::SHORT_COMMIT, rom_title),
        native_options,
        Box::new(move |cc| Ok(Box::new(Renderer::new(cc, display_rx, dbg_req_tx, dbg_resp_rx)))),
    );
}
