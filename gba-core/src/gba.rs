use crate::arm7tdmi::cpu::Cpu;
use crate::cartridge::database::TITLE_DATABASE;
use crate::cartridge::storage::BackupType;
use crate::memory::mmio::Mmio;
use crate::script::engine::ScriptEngine;
use std::path::Path;
use tracing::{error, info};

pub struct Gba {
    pub cpu: Cpu,
    pub script_engine: Option<ScriptEngine>,
    pub rom_title: String,
    pub crc32: String,
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
                error!(target: "cartridge",
                    "CRC32 '{}' not found in database, using default save type and title.",
                    crc32
                );
                (BackupType::Sram, false, game_title.clone())
            });
        info!(target: "cartridge", "Save Type: {}", save_type);
        info!(target: "cartridge", "Game Title: {}", rom_title);

        let mut mmio = Mmio::new(save_type, has_rtc);
        mmio.load(0x00000000, include_bytes!("../../external/gba_bios.bin"));

        // Load ROM into memory
        mmio.load(0x08000000, &rom_data);

        let cpu = Cpu::new(&elf_data, mmio);

        Gba {
            cpu,
            script_engine: None,
            rom_title,
            crc32,
        }
    }

    pub fn load_rhai_script(&mut self, path: String) {
        let path = Path::new(&path);

        let mut engine = ScriptEngine::new();
        engine.load_script(path);

        self.script_engine = Some(engine);

        info!(target: "rhai", "Successfully loaded script: {}", path.display());
    }

    pub fn try_execute_breakpoint(&mut self, address: u32, pc: u32) {
        if let Some(engine) = &mut self.script_engine {
            engine.handle_breakpoint(address, pc, &mut self.cpu);
        }
    }

    pub fn save_devices(&self, base_path: &Path) {
        let storage_data = self.cpu.mmio.storage_chip.aggregate_storage();
        let storage_path = base_path.join(&self.crc32);
        std::fs::create_dir_all(&storage_path).expect("Failed to create save directory");

        let storage_path = storage_path.join("storage.bin");

        if let Err(e) = std::fs::write(&storage_path, &storage_data) {
            error!(target: "storage", "Failed to save data: {}", e);
        } else {
            info!(target: "storage", "Data saved to {}", storage_path.display());
        }
    }

    pub fn load_devices(&mut self, base_path: &Path) {
        let storage_path = base_path.join(&self.crc32);
        std::fs::create_dir_all(&storage_path).expect("Failed to create save directory");

        let storage_path = storage_path.join("storage.bin");

        match std::fs::read(&storage_path) {
            Ok(data) => {
                self.cpu.mmio.storage_chip.load_storage(&data);
                info!(target: "storage", "Save data loaded from {}", storage_path.display());
            }
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => {
                error!(target: "storage", "Failed to read save data from {}", storage_path.display());
            }
            // File not found means no save data exists, which is fine.
            _ => {}
        }
    }
}
