use crate::arm7tdmi::cpu::Cpu;
use crate::cartridge::database::TITLE_DATABASE;
use crate::cartridge::storage::BackupType;
use crate::memory::mmio::Mmio;
use crate::script::engine::ScriptEngine;
use log::info;
use std::path::Path;

pub struct Gba {
    pub cpu: Cpu,
    pub script_engine: Option<ScriptEngine>,
    pub rom_title: String,
}

impl Gba {
    pub fn new(rom_data: &[u8], elf_data: &[u8]) -> Self {
        let game_title = String::from_utf8_lossy(&rom_data[0xa0..0xa0 + 12]).to_string(); // use as backup

        let crc32 = crc32fast::hash(rom_data);
        let crc32 = format!("{:08x}", crc32);

        let (save_type, has_rtc, rom_title) = TITLE_DATABASE
            .get(&crc32)
            .map(|&(backup_type, has_rtc, game_title)| (backup_type.into(), has_rtc, game_title.to_string()))
            .unwrap_or_else(|| {
                eprintln!(
                    "CRC32 '{}' not found in database, using default save type and title.",
                    crc32
                );
                (BackupType::Sram, false, game_title.clone())
            });
        println!("Save Type: {}", save_type);
        println!("Game Title: {}", rom_title);

        let mut mmio = Mmio::new(save_type, has_rtc);
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
