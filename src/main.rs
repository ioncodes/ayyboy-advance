#![feature(new_uninit)]
#![feature(if_let_guard)]
#![feature(let_chains)]

mod arm7tdmi;
mod frontend;
mod memory;
mod tests;
mod video;

use arm7tdmi::cpu::Cpu;
use arm7tdmi::decoder::Register;
use arm7tdmi::mode::ProcessorMode;
use eframe::NativeOptions;
use egui::ViewportBuilder;
use frontend::dbg::event::{RequestEvent, ResponseEvent};
use frontend::dbg::widgets;
use frontend::renderer::{Renderer, SCALE};
use memory::mmio::Mmio;

use crossbeam_channel::{self, Receiver, Sender};
use video::{Frame, SCREEN_HEIGHT, SCREEN_WIDTH};

const ARM_TEST: &[u8] = include_bytes!("../external/gba-tests/arm/arm.gba");
// const ARM_TEST: &[u8] = include_bytes!("../external/gba-div-test/out/rom.gba"); // just a div test
// const ARM_TEST: &[u8] = include_bytes!("../external/gba-psr-test/out/rom.gba"); // just a cpsr bank test
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/panda.gba"); // works
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/methharold.gba"); // works
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/gang.gba"); // works
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/gang-ldmstm.gba");
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/armfuck.gba");
const BIOS: &[u8] = include_bytes!("../external/gba_bios.bin");

fn process_debug_events(
    cpu: &Cpu, mmio: &Mmio, dbg_req_rx: &Receiver<RequestEvent>, dbg_resp_tx: &Sender<ResponseEvent>,
) {
    let _ = dbg_req_rx
        .try_recv() // check for new requests
        .map(|event| match event {
            RequestEvent::UpdateCpu => {
                let _ = dbg_resp_tx.send(ResponseEvent::Cpu(widgets::cpu::Cpu {
                    registers: cpu.registers.r,
                    cpsr: cpu.registers.cpsr,
                }));
            }
            RequestEvent::UpdateMemory => {
                let mut memory = unsafe {
                    let memory = Box::<[u8; 0x0FFFFFFF + 1]>::new_zeroed();
                    memory.assume_init()
                };
                memory[..=0x04FFFFFF].copy_from_slice(&mmio.internal_memory[..]);
                memory[0x05000000..=0x07FFFFFF].copy_from_slice(&mmio.ppu.vram[..]);
                memory[0x08000000..=0x0FFFFFFF].copy_from_slice(&mmio.external_memory[..]);
                let _ = dbg_resp_tx.send(ResponseEvent::Memory(memory));
            }
        });
}

fn main() {
    env_logger::builder().format_timestamp(None).init();

    let (display_tx, display_rx): (Sender<Frame>, Receiver<Frame>) = crossbeam_channel::bounded(1);
    let (dbg_req_tx, dbg_req_rx): (Sender<RequestEvent>, Receiver<RequestEvent>) = crossbeam_channel::bounded(5);
    let (dbg_resp_tx, dbg_resp_rx): (Sender<ResponseEvent>, Receiver<ResponseEvent>) = crossbeam_channel::bounded(5);

    std::thread::spawn(move || {
        let mut mmio = Mmio::new();
        mmio.load(0x00000000, BIOS); // bios addr
        mmio.load(0x08000000, ARM_TEST); // gamepak addr

        let mut cpu = Cpu::new();
        // State for skipping BIOS, https://problemkaputt.de/gbatek.htm#biosramusage
        cpu.set_processor_mode(ProcessorMode::Irq);
        cpu.write_register(&Register::R13, 0x03007fa0);
        cpu.set_processor_mode(ProcessorMode::Supervisor);
        cpu.write_register(&Register::R13, 0x03007fe0);
        cpu.set_processor_mode(ProcessorMode::User);
        cpu.write_register(&Register::R13, 0x03007f00);
        cpu.set_processor_mode(ProcessorMode::System);
        cpu.write_register(&Register::R13, 0x03007f00);
        cpu.write_register(&Register::R15, 0x08000000);

        let mut frame_rendered = false;

        loop {
            cpu.tick(&mut mmio);
            mmio.tick_components();

            process_debug_events(&cpu, &mmio, &dbg_req_rx, &dbg_resp_tx);

            if mmio.ppu.scanline == 160 && !frame_rendered {
                let _ = display_tx.send(mmio.ppu.get_frame());
                frame_rendered = true;
            } else if mmio.ppu.scanline == 0 && frame_rendered {
                frame_rendered = false;
            }
        }
    });

    let native_options = NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([(SCREEN_WIDTH * SCALE) as f32, (SCREEN_HEIGHT * SCALE) as f32])
            .with_resizable(false),
        vsync: false,
        ..Default::default()
    };

    let _ = eframe::run_native(
        "ayyboy advance",
        native_options,
        Box::new(move |cc| Ok(Box::new(Renderer::new(cc, display_rx, dbg_req_tx, dbg_resp_rx)))),
    );
}
