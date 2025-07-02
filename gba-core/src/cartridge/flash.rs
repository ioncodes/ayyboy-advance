use crate::cartridge::storage::BackupType;
use crate::cartridge::StorageChip;
use crate::memory::device::Addressable;

const FLASH_512K_SIZE: u32 = 0x10000; // 64 KiB
const FLASH_1M_SIZE: u32 = 0x20000; // 128 KiB

pub struct Flash {
    flash: Vec<u8>,
    backup_type: BackupType,
    boundary: u32,
    _has_rtc: bool,
}

impl Flash {
    pub fn new(backup_type: BackupType, has_rtc: bool) -> Self {
        let flash_size = if backup_type == BackupType::Flash512k {
            FLASH_512K_SIZE
        } else {
            FLASH_1M_SIZE
        };

        Flash {
            flash: vec![0; flash_size as usize],
            backup_type,
            boundary: flash_size,
            _has_rtc: has_rtc,
        }
    }
}

impl Addressable for Flash {
    fn read(&self, addr: u32) -> u8 {
        match addr {
            0x0E000000 => self.backup_type.manufacturer_id(),
            0x0E000001 => self.backup_type.device_id(),
            0x0E000002..=0x0FFFFFFF => {
                let addr = (addr - 0x0E000000) % self.boundary;
                self.flash[addr as usize]
            }
            _ => unreachable!("Invalid address for Flash read: {:08x}", addr),
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0x0E000002..=0x0FFFFFFF => {
                let addr = (addr - 0x0E000000) % self.boundary;
                self.flash[addr as usize] = value;
            }
            _ => {}
        }
    }
}

impl StorageChip for Flash {
    fn size(&self) -> usize {
        self.boundary as usize
    }

    fn backup_type(&self) -> BackupType {
        self.backup_type
    }
}
