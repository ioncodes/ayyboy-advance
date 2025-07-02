use crate::cartridge::storage::BackupType;
use crate::cartridge::StorageChip;
use crate::memory::device::Addressable;

const EEPROM_BASE: u32 = 0x0D000000;

const EEPROM_4K_SIZE: u32 = 0x200; // 512 bytes
const EEPROM_64K_SIZE: u32 = 0x10000; // 64 KiB

pub struct Eeprom {
    pub eeprom: Vec<u8>,
    pub backup_type: BackupType,
    boundary: u32,
}

impl Eeprom {
    pub fn new(backup_type: BackupType) -> Self {
        let eeprom_size = if backup_type == BackupType::Eeprom4k {
            EEPROM_4K_SIZE
        } else {
            EEPROM_64K_SIZE
        };

        Eeprom {
            eeprom: vec![0xFF; eeprom_size as usize],
            backup_type,
            boundary: eeprom_size,
        }
    }
}

impl Addressable for Eeprom {
    fn read(&self, addr: u32) -> u8 {
        //println!("Reading from EEPROM at address: {:08x}", addr);
        0xFF
    }

    fn write(&mut self, addr: u32, value: u8) {
        println!("Writing to EEPROM at address: {:08x} with value: {:08b}", addr, value);
        self.eeprom[(addr - EEPROM_BASE) as usize % self.boundary as usize] = value;
    }
}

impl StorageChip for Eeprom {
    fn size(&self) -> usize {
        self.boundary as usize
    }

    fn backup_type(&self) -> BackupType {
        self.backup_type
    }
}
