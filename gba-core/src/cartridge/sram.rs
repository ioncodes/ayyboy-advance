use crate::cartridge::storage::BackupType;
use crate::cartridge::StorageChip;
use crate::memory::device::Addressable;

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
}
