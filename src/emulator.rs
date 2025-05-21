use crate::arm7tdmi::cpu::Cpu;
use crate::arm7tdmi::decoder::{Instruction, Register};
use crate::arm7tdmi::mode::ProcessorMode;
use crate::frontend::dbg::widgets;
use crate::frontend::dbg::widgets::disasm::DecodedInstruction;
use crate::frontend::event::{RequestEvent, ResponseEvent};
use crate::memory::mmio::Mmio;
use crate::script::engine::ScriptEngine;
use crate::video::Frame;
use crossbeam_channel::{Receiver, Sender};
use lazy_static::lazy_static;
use spdlog::info;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::sync::Mutex;

lazy_static! {
    pub static ref BREAKPOINTS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
}

pub struct Emulator {
    pub cpu: Cpu,
    pub script_engine: ScriptEngine,
    pub display_tx: Sender<Frame>,
    pub dbg_req_rx: Receiver<RequestEvent>,
    pub dbg_resp_tx: Sender<ResponseEvent>,
    current_cycles: usize,
}

impl Emulator {
    pub fn new(
        display_tx: Sender<Frame>, dbg_req_rx: Receiver<RequestEvent>, dbg_resp_tx: Sender<ResponseEvent>,
        script_path: Option<String>, rom_path: String,
    ) -> Self {
        let mut mmio = Mmio::new();
        mmio.load(0x00000000, include_bytes!("../external/gba_bios.bin"));

        // Load ROM from file
        let mut rom_data = Vec::new();
        let mut rom_file = File::open(&rom_path).expect("Failed to open ROM file");
        rom_file.read_to_end(&mut rom_data).expect("Failed to read ROM file");
        mmio.load(0x08000000, &rom_data);

        // Check for corresponding ELF file (for symbolizer)
        let elf_path = rom_path.replace(".gba", ".elf");
        let elf_data = if Path::new(&elf_path).exists() {
            let mut elf_file = File::open(&elf_path).expect("Failed to open ELF file");
            let mut data = Vec::new();
            elf_file.read_to_end(&mut data).expect("Failed to read ELF file");
            data
        } else {
            Vec::new()
        };

        let mut cpu = Cpu::new(&elf_data, mmio);
        let mut script_engine = ScriptEngine::new();

        // Load script if provided
        if let Some(path) = script_path {
            let path = Path::new(&path);
            if script_engine.load_script(path) {
                info!("Successfully loaded script: {}", path.display());
            }
        }

        // Initialize CPU state (post BIOS)
        cpu.set_processor_mode(ProcessorMode::Irq);
        cpu.write_register(&Register::R13, 0x03007fa0);
        cpu.set_processor_mode(ProcessorMode::Supervisor);
        cpu.write_register(&Register::R13, 0x03007fe0);
        cpu.set_processor_mode(ProcessorMode::User);
        cpu.write_register(&Register::R13, 0x03007f00);
        cpu.set_processor_mode(ProcessorMode::System);
        cpu.write_register(&Register::R13, 0x03007f00);
        cpu.write_register(&Register::R14, 0x08000000);
        cpu.write_register(&Register::R15, 0x08000000);

        Self {
            cpu,
            script_engine,
            display_tx,
            dbg_req_rx,
            dbg_resp_tx,
            current_cycles: 0,
        }
    }

    pub fn run(&mut self) {
        let mut frame_rendered = false;
        let mut tick = false;
        let mut step = false;

        loop {
            match self.process_debug_events() {
                EventResult::Break => tick = false,
                EventResult::Continue => tick = true,
                EventResult::Step if !tick => {
                    step = true;
                }
                _ => (),
            }

            if tick || step {
                self.do_step(&mut tick);
            }

            if step {
                step = false;
            }

            if self.cpu.mmio.ppu.scanline.0 == 160 && !frame_rendered {
                let _ = self.display_tx.send(self.cpu.mmio.ppu.get_frame());
                frame_rendered = true;
            } else if self.cpu.mmio.ppu.scanline.0 == 0 && frame_rendered {
                frame_rendered = false;
            }
        }
    }

    fn process_debug_events(&mut self) -> EventResult {
        self.dbg_req_rx
            .try_recv()
            .map(|event| match event {
                RequestEvent::UpdateCpu => {
                    let _ = self.dbg_resp_tx.send(ResponseEvent::Cpu(widgets::cpu::Cpu {
                        registers: self.cpu.registers.r,
                        cpsr: self.cpu.registers.cpsr,
                    }));
                    EventResult::None
                }
                RequestEvent::UpdateMemory => {
                    let mut memory = unsafe {
                        let memory = Box::<[u8; 0x0FFFFFFF + 1]>::new_zeroed();
                        memory.assume_init()
                    };
                    memory[..=0x04FFFFFF].copy_from_slice(&self.cpu.mmio.internal_memory[..]);
                    memory[0x05000000..=0x07FFFFFF].copy_from_slice(&self.cpu.mmio.ppu.vram[..]);
                    memory[0x08000000..=0x0FFFFFFF].copy_from_slice(&self.cpu.mmio.external_memory[..]);
                    let _ = self.dbg_resp_tx.send(ResponseEvent::Memory(memory));
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
                    let base = base.unwrap_or(if let Some((_, state)) = self.cpu.pipeline.peek_fetch() {
                        state.pc
                    } else {
                        self.cpu.read_register(&Register::R15)
                    });
                    let mut disasm: Vec<DecodedInstruction> = Vec::new();
                    for addr in 0..count {
                        let addr = base + (addr * if self.cpu.is_thumb() { 2 } else { 4 });
                        let opcode = self.cpu.mmio.read_u32(addr);
                        match Instruction::decode(opcode, self.cpu.is_thumb()) {
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
                    let _ = self.dbg_resp_tx.send(ResponseEvent::Disassembly(
                        base,
                        self.cpu.read_register(&Register::R15),
                        disasm,
                    ));
                    EventResult::None
                }
                RequestEvent::UpdateKeyState(state) => {
                    for (key, pressed) in state {
                        self.cpu.mmio.joypad.set_key_state(key, pressed);
                    }
                    EventResult::None
                }
            })
            .unwrap_or(EventResult::None)
    }

    fn do_step(&mut self, tick: &mut bool) {
        loop {
            if let Some(_) = self.do_tick(tick) {
                break;
            }
        }
    }

    fn do_tick(&mut self, tick: &mut bool) -> Option<Instruction> {
        let mut executed_instr: Option<Instruction> = None;

        if let Some((instr, state)) = self.cpu.tick(Some(&mut self.script_engine)) {
            if BREAKPOINTS
                .lock()
                .unwrap()
                .contains(&(state.pc + if self.cpu.is_thumb() { 2 } else { 4 }))
            {
                *tick = false;
            }
            executed_instr = Some(instr);
            self.current_cycles += 1; // TODO: actually track it
        }

        if self.current_cycles > 5 {
            self.current_cycles = 0;
            self.cpu.mmio.tick_components();
        }

        executed_instr
    }
}

pub enum EventResult {
    Break,
    Continue,
    Step,
    None,
}
