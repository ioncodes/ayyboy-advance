use crate::cartridge::storage::BackupType;
use crate::cartridge::StorageChip;
use crate::memory::device::Addressable;

const FLASH_512K_SIZE: u32 = 0x10000; // 64 KiB
const FLASH_1M_SIZE: u32 = 0x20000; // 128 KiB

pub struct Flash {
    pub external_memory: Box<[u8; FLASH_1M_SIZE as usize]>,
    pub backup_type: BackupType,
    boundary: u32,
    _has_rtc: bool,
}

impl Flash {
    pub fn new(backup_type: BackupType, has_rtc: bool) -> Self {
        let external_memory = Box::<[u8; FLASH_1M_SIZE as usize]>::new_zeroed();

        Flash {
            external_memory: unsafe { external_memory.assume_init() },
            backup_type,
            boundary: if matches!(backup_type, BackupType::Flash512k { .. }) {
                FLASH_512K_SIZE
            } else {
                FLASH_1M_SIZE
            },
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
                self.external_memory[addr as usize]
            }
            _ => unreachable!("Invalid address for Flash read: {:08x}", addr),
        }
    }

    fn write(&mut self, addr: u32, value: u8) {
        match addr {
            0x0E000002..=0x0FFFFFFF => {
                let addr = (addr - 0x0E000000) % self.boundary;
                self.external_memory[addr as usize] = value;
            }
            _ => {}
        }
    }
}

impl StorageChip for Flash {
    fn size(&self) -> usize {
        self.boundary as usize
    }
}
