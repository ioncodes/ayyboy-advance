use crossbeam_channel::{Receiver, Sender};
use gba_core::arm7tdmi::cpu::Cpu;
use gba_core::arm7tdmi::decoder::{Instruction, Register};
use gba_core::arm7tdmi::mode::ProcessorMode;
use gba_core::memory::mmio::Mmio;
use gba_core::script::engine::ScriptEngine;
use gba_core::video::{Frame, FRAME_0_ADDRESS, FRAME_1_ADDRESS};
use lazy_static::lazy_static;
use log::info;
use std::fs::File;
use std::io::{Cursor, Read};
use std::path::Path;
use std::sync::Mutex;
use zip::ZipArchive;

use crate::dbg::widgets;
use crate::dbg::widgets::disasm::DecodedInstruction;
use crate::dbg::widgets::ppu::PpuRegisters;
use crate::event::{RequestEvent, ResponseEvent};

lazy_static! {
    pub static ref BREAKPOINTS: Mutex<Vec<u32>> = Mutex::new(Vec::new());
}

pub struct Emulator {
    pub cpu: Cpu,
    pub script_engine: ScriptEngine,
    pub display_tx: Sender<Frame>,
    pub dbg_req_rx: Receiver<RequestEvent>,
    pub dbg_resp_tx: Sender<ResponseEvent>,
    pub rom_title: String,
}

impl Emulator {
    pub fn new(
        display_tx: Sender<Frame>, dbg_req_rx: Receiver<RequestEvent>, dbg_resp_tx: Sender<ResponseEvent>,
        script_path: Option<String>, rom_path: String,
    ) -> Self {
        let mut mmio = Mmio::new();
        mmio.load(0x00000000, include_bytes!("../../external/gba_bios.bin"));

        // Load ROM from file
        let mut rom_data = Vec::new();
        let mut rom_file = File::open(&rom_path).expect("Failed to open ROM file");
        rom_file.read_to_end(&mut rom_data).expect("Failed to read ROM file");

        // If it's a ZIP file, extract the ROM
        if rom_path.ends_with(".zip") {
            rom_data = Self::unzip_archive(&rom_data);
        }

        // Extract ROM title
        let rom_title = String::from_utf8_lossy(&rom_data[0xa0..0xa0 + 12]).to_string();

        // Load ROM into memory
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

        let cpu = Cpu::new(&elf_data, mmio);
        let mut script_engine = ScriptEngine::new();

        // Load script if provided
        if let Some(path) = script_path {
            let path = Path::new(&path);
            script_engine.load_script(path);
            info!("Successfully loaded script: {}", path.display());
        }

        Self {
            cpu,
            script_engine,
            display_tx,
            dbg_req_rx,
            dbg_resp_tx,
            rom_title,
        }
    }

    #[allow(dead_code)]
    pub fn skip_bios(&mut self) {
        // Initialize CPU state (post BIOS)
        self.cpu.set_processor_mode(ProcessorMode::Irq);
        self.cpu.write_register(&Register::R13, 0x03007fa0);
        self.cpu.set_processor_mode(ProcessorMode::Supervisor);
        self.cpu.write_register(&Register::R13, 0x03007fe0);
        self.cpu.set_processor_mode(ProcessorMode::User);
        self.cpu.write_register(&Register::R13, 0x03007f00);
        self.cpu.set_processor_mode(ProcessorMode::System);
        self.cpu.write_register(&Register::R13, 0x03007f00);
        self.cpu.write_register(&Register::R14, 0x08000000);
        self.cpu.write_register(&Register::R15, 0x08000000);
        self.cpu.mmio.io_postflg.write(0x01);
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
                self.do_tick(&mut tick);
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
                        dma: self.cpu.mmio.dma,
                        timers: self.cpu.mmio.timers,
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
                    let base = base.unwrap_or(if let Some(state) = self.cpu.pipeline.peek_fetch() {
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
                RequestEvent::UpdatePpu => {
                    let _ = self.dbg_resp_tx.send(ResponseEvent::Ppu(
                        vec![
                            self.cpu.mmio.ppu.get_background_frame(3, FRAME_0_ADDRESS),
                            self.cpu.mmio.ppu.get_background_frame(3, FRAME_1_ADDRESS),
                            self.cpu.mmio.ppu.get_background_frame(4, FRAME_0_ADDRESS),
                            self.cpu.mmio.ppu.get_background_frame(4, FRAME_1_ADDRESS),
                            self.cpu.mmio.ppu.get_background_frame(5, FRAME_0_ADDRESS),
                            self.cpu.mmio.ppu.get_background_frame(5, FRAME_1_ADDRESS),
                        ],
                        self.cpu.mmio.ppu.render_tileset(),
                        [
                            self.cpu.mmio.ppu.render_tilemap(self.cpu.mmio.ppu.bg_cnt[0].value()),
                            self.cpu.mmio.ppu.render_tilemap(self.cpu.mmio.ppu.bg_cnt[1].value()),
                            self.cpu.mmio.ppu.render_tilemap(self.cpu.mmio.ppu.bg_cnt[2].value()),
                            self.cpu.mmio.ppu.render_tilemap(self.cpu.mmio.ppu.bg_cnt[3].value()),
                        ],
                        Vec::from(self.cpu.mmio.ppu.fetch_palette()),
                        PpuRegisters {
                            disp_cnt: *self.cpu.mmio.ppu.disp_cnt.value(),
                            disp_stat: *self.cpu.mmio.ppu.disp_stat.value(),
                            bg_cnt: self.cpu.mmio.ppu.bg_cnt.map(|bg| *bg.value()),
                            bg_vofs: self.cpu.mmio.ppu.bg_vofs.map(|bg| *bg.value()),
                            bg_hofs: self.cpu.mmio.ppu.bg_hofs.map(|bg| *bg.value()),
                        },
                        self.cpu.mmio.ppu.create_sprite_debug_map(),
                    ));
                    EventResult::None
                }
            })
            .unwrap_or(EventResult::None)
    }

    fn do_tick(&mut self, tick: &mut bool) -> Option<Instruction> {
        let mut executed_instr: Option<Instruction> = None;

        if let Ok((instr, state)) = self.cpu.tick() {
            if BREAKPOINTS
                .lock()
                .unwrap()
                .contains(&(state.pc + if self.cpu.is_thumb() { 2 } else { 4 }))
            {
                *tick = false;
            }

            self.script_engine.handle_breakpoint(state.pc, state.pc, &mut self.cpu);
            for addr in self.cpu.mmio.last_rw_addr.clone() {
                self.script_engine.handle_breakpoint(addr, state.pc, &mut self.cpu);
            }

            executed_instr = Some(instr);
        }

        self.cpu.mmio.tick_components();

        executed_instr
    }

    fn unzip_archive(buffer: &[u8]) -> Vec<u8> {
        let mut archive = ZipArchive::new(Cursor::new(buffer)).unwrap();

        let gba_index = (0..archive.len())
            .filter(|&i| archive.by_index(i).unwrap().name().contains(".gba"))
            .next()
            .unwrap_or_else(|| panic!("No .gba file found in archive"));

        let mut file = archive.by_index(gba_index).unwrap();
        let mut buffer = Vec::with_capacity(file.size() as usize);
        let _ = file.read_to_end(&mut buffer).unwrap();

        buffer
    }
}

pub enum EventResult {
    Break,
    Continue,
    Step,
    None,
}
