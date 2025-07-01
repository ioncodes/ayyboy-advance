use crate::cartridge::storage::BackupType;
use crate::cartridge::StorageChip;
use crate::memory::device::Addressable;

const SRAM_SIZE: u32 = 0x8000; // 32 KiB

pub struct Flash {
    pub external_memory: Box<[u8; SRAM_SIZE as usize]>,
    pub backup_type: BackupType,
}

impl Flash {
    pub fn new(backup_type: BackupType) -> Self {
        let external_memory = Box::<[u8; SRAM_SIZE as usize]>::new_zeroed();
        Flash {
            external_memory: unsafe { external_memory.assume_init() },
            backup_type,
        }
    }
}

impl Addressable for Flash {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x0E000000 => self.backup_type.manufacturer_id(),
            0x0E000001 => self.backup_type.device_id(),
            0x0E000002..=0x0FFFFFFF => {
                // GamePak SRAM – mirrors every 32 KiB in 0x0E000000‑0x0FFFFFFF
                let addr = (addr - 0x0E000000) % SRAM_SIZE;
                self.external_memory[addr as usize]
            }
            _ => unreachable!(),
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0x0E000002..=0x0FFFFFFF => {
                // GamePak SRAM – mirrors every 32 KiB in 0x0E000000‑0x0FFFFFFF
                let addr = (addr - 0x0E000000) % SRAM_SIZE;
                self.external_memory[addr as usize] = value;
            }
            _ => {}
        }
    }
}

impl StorageChip for Flash {
    fn size(&self) -> usize {
        SRAM_SIZE as usize
    }
}
