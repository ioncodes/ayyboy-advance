use tracing::debug;

use crate::cartridge::StorageChip;
use crate::cartridge::storage::BackupType;
use crate::memory::device::{Addressable, Saveable};

const SRAM_SIZE: u32 = 0x8000; // 32 KiB

pub struct Sram {
    sram: Vec<u8>,
    backup_type: BackupType,
}

impl Sram {
    pub fn new() -> Self {
        Sram {
            sram: vec![0; SRAM_SIZE as usize],
            backup_type: BackupType::Sram,
        }
    }
}

impl Addressable for Sram {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x0E000000..=0x0FFFFFFF => {
                // GamePak SRAM – mirrors every 32 KiB in 0x0E000000‑0x0FFFFFFF
                let addr = (addr - 0x0E000000) % SRAM_SIZE;
                self.sram[addr as usize]
            }
            _ => unreachable!(),
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0x0E000000..=0x0FFFFFFF => {
                // GamePak SRAM – mirrors every 32 KiB in 0x0E000000‑0x0FFFFFFF
                let addr = (addr - 0x0E000000) % SRAM_SIZE;
                self.sram[addr as usize] = value;
            }
            _ => unreachable!(),
        }
    }
}

impl StorageChip for Sram {
    fn size(&self) -> usize {
        SRAM_SIZE as usize
    }

    fn backup_type(&self) -> BackupType {
        self.backup_type
    }

    fn backing_storage(&self) -> Vec<u8> {
        self.sram.clone()
    }
}

impl Saveable for Sram {
    fn aggregate_storage(&self) -> Vec<u8> {
        self.sram.clone()
    }

    fn load_storage(&mut self, data: &[u8]) {
        if data.len() != SRAM_SIZE as usize {
            panic!("Invalid SRAM data size: expected {}, got {}", SRAM_SIZE, data.len());
        }

        debug!(target: "storage", "Loading SRAM with {} bytes", data.len());
        self.sram.copy_from_slice(data);
    }
}
