use crate::arm7tdmi::cpu::Cpu;
use crate::cartridge::database::TITLE_DATABASE;
use crate::memory::mmio::Mmio;
use crate::script::engine::ScriptEngine;
use log::{error, info};
use std::path::Path;

pub struct Gba {
    pub cpu: Cpu,
    pub script_engine: Option<ScriptEngine>,
    pub rom_title: String,
}

impl Gba {
    pub fn new(rom_data: &[u8], elf_data: &[u8]) -> Self {
        // Extract game code
        let game_code = String::from_utf8_lossy(&rom_data[0xac..0xac + 4]).to_string();
        let game_title = String::from_utf8_lossy(&rom_data[0xa0..0xa0 + 12]).to_string(); // use as backup
        info!("Game Code: {}", game_code);
        info!("Game Title: {}", game_title);

        let (save_type, rom_title) = TITLE_DATABASE
            .get(&game_code)
            .map(|&(st, rt)| (st.into(), rt.to_string()))
            .unwrap_or_else(|| {
                error!(
                    "Game code '{}' not found in database, using default save type and title.",
                    game_code
                );
                (0.into(), game_title.clone())
            });
        info!("Save Type: {}", save_type);

        let mut mmio = Mmio::new(save_type);
        mmio.load(0x00000000, include_bytes!("../../external/gba_bios.bin"));

        // Load ROM into memory
        mmio.load(0x08000000, &rom_data);

        let cpu = Cpu::new(&elf_data, mmio);

        Gba {
            cpu,
            script_engine: None,
            rom_title,
        }
    }

    pub fn load_rhai_script(&mut self, path: String) {
        let path = Path::new(&path);

        let mut engine = ScriptEngine::new();
        engine.load_script(path);

        self.script_engine = Some(engine);

        info!("Successfully loaded script: {}", path.display());
    }

    pub fn try_execute_breakpoint(&mut self, address: u32, pc: u32) {
        if let Some(engine) = &mut self.script_engine {
            engine.handle_breakpoint(address, pc, &mut self.cpu);
        }
    }
}
