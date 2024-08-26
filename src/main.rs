#![feature(new_uninit)]
#![feature(if_let_guard)]
#![feature(let_chains)]

mod arm7tdmi;
mod frontend;
mod input;
mod memory;
mod tests;
mod video;

use std::sync::Mutex;

use arm7tdmi::cpu::Cpu;
use arm7tdmi::decoder::{Instruction, Register};
use arm7tdmi::mode::ProcessorMode;
use eframe::NativeOptions;
use egui::ViewportBuilder;
use frontend::dbg::widgets;
use frontend::dbg::widgets::disasm::DecodedInstruction;
use frontend::event::{RequestEvent, ResponseEvent};
use frontend::renderer::{Renderer, SCALE};
use lazy_static::lazy_static;
use memory::mmio::Mmio;

use crossbeam_channel::{self, Receiver, Sender};
use video::{Frame, SCREEN_HEIGHT, SCREEN_WIDTH};

// const ARM_TEST: &[u8] = include_bytes!("../external/gba-tests/arm/arm.gba");
const ARM_TEST: &[u8] = include_bytes!("../external/armwrestler-gba-fixed/armwrestler-gba-fixed.gba");
// const ARM_TEST: &[u8] = include_bytes!("../external/gba-div-test/out/rom.gba"); // just a div test
// const ARM_TEST: &[u8] = include_bytes!("../external/gba-psr-test/out/rom.gba"); // just a cpsr bank test
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/panda.gba"); // works
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/methharold.gba"); // works
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/gang.gba"); // works
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/gang-ldmstm.gba");
// const ARM_TEST: &[u8] = include_bytes!("../external/discord/armfuck.gba");
const BIOS: &[u8] = include_bytes!("../external/gba_bios.bin");

lazy_static! {
    // breakspoints
    static ref BREAKPOINTS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
}

enum EventResult {
    Break,
    Continue,
    Step,
    None,
}

fn process_debug_events(
    cpu: &Cpu, mmio: &mut Mmio, dbg_req_rx: &Receiver<RequestEvent>, dbg_resp_tx: &Sender<ResponseEvent>,
) -> EventResult {
    dbg_req_rx
        .try_recv() // check for new requests
        .map(|event| match event {
            RequestEvent::UpdateCpu => {
                let _ = dbg_resp_tx.send(ResponseEvent::Cpu(widgets::cpu::Cpu {
                    registers: cpu.registers.r,
                    cpsr: cpu.registers.cpsr,
                }));
                EventResult::None
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
                EventResult::None
            }
            RequestEvent::Break => EventResult::Break,
            RequestEvent::Run => EventResult::Continue,
            RequestEvent::Step => EventResult::Step,
            RequestEvent::AddBreakpoint(addr) => {
                BREAKPOINTS.lock().unwrap().push(addr);
                EventResult::None
            }
            RequestEvent::RemoveBreakpoint(addr) => {
                let mut breakpoints = BREAKPOINTS.lock().unwrap();
                if let Some(index) = breakpoints.iter().position(|&x| x == addr) {
                    breakpoints.remove(index);
                }
                EventResult::None
            }
            RequestEvent::UpdateDisassembly(base, count) => {
                // decoded instruction would never be available here
                let base = base.unwrap_or(if let Some((_, state)) = cpu.pipeline.peek_fetch() {
                    state.pc
                } else {
                    cpu.read_register(&Register::R15)
                });
                let mut disasm: Vec<DecodedInstruction> = Vec::new();
                for addr in 0..count {
                    let addr = base + (addr * if cpu.is_thumb() { 2 } else { 4 });
                    let opcode = mmio.read_u32(addr);
                    match Instruction::decode(opcode, cpu.is_thumb()) {
                        Ok(instr) => disasm.push(DecodedInstruction {
                            addr,
                            instr: format!("{}", instr),
                        }),
                        Err(_) => disasm.push(DecodedInstruction {
                            addr,
                            instr: "???".to_string(),
                        }),
                    }
                }
                let _ = dbg_resp_tx.send(ResponseEvent::Disassembly(
                    base,
                    cpu.read_register(&Register::R15),
                    disasm,
                ));
                EventResult::None
            }
            RequestEvent::UpdateKeyState(state) => {
                for (key, pressed) in state {
                    mmio.joypad.set_key_state(key, pressed);
                }
                EventResult::None
            }
        })
        .unwrap_or(EventResult::None)
}

fn main() {
    env_logger::builder().format_timestamp(None).init();

    let (display_tx, display_rx): (Sender<Frame>, Receiver<Frame>) = crossbeam_channel::bounded(1);
    let (dbg_req_tx, dbg_req_rx): (Sender<RequestEvent>, Receiver<RequestEvent>) = crossbeam_channel::bounded(25);
    let (dbg_resp_tx, dbg_resp_rx): (Sender<ResponseEvent>, Receiver<ResponseEvent>) = crossbeam_channel::bounded(25);

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
        let mut tick = false;
        let mut step = false;

        let do_tick = |cpu: &mut Cpu, mmio: &mut Mmio, tick_ref: &mut bool| -> Option<Instruction> {
            let mut executed_instr: Option<Instruction> = None;
            if let Some((instr, state)) = cpu.tick(mmio) {
                if BREAKPOINTS
                    .lock()
                    .unwrap()
                    .contains(&(state.pc + if cpu.is_thumb() { 2 } else { 4 }))
                {
                    *tick_ref = false;
                }
                executed_instr = Some(instr);
            }

            mmio.tick_components();

            executed_instr
        };

        let do_step = |cpu: &mut Cpu, mmio: &mut Mmio, tick: &mut bool| loop {
            if let Some(_) = do_tick(cpu, mmio, tick) {
                break;
            }
        };

        loop {
            match process_debug_events(&cpu, &mut mmio, &dbg_req_rx, &dbg_resp_tx) {
                EventResult::Break => tick = false,
                EventResult::Continue => tick = true,
                EventResult::Step if !tick => {
                    step = true;
                }
                _ => (),
            }

            if tick || step {
                do_step(&mut cpu, &mut mmio, &mut tick);
            }

            if step {
                step = false;
            }

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
        default_theme: eframe::Theme::Dark,
        ..Default::default()
    };

    let _ = eframe::run_native(
        "ayyboy advance",
        native_options,
        Box::new(move |cc| Ok(Box::new(Renderer::new(cc, display_rx, dbg_req_tx, dbg_resp_rx)))),
    );
}
