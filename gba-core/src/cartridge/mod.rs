use crate::cartridge::storage::BackupType;
use crate::memory::device::Addressable;

pub mod database;
pub mod eeprom;
pub mod flash;
pub mod sram;
pub mod storage;

pub trait StorageChip: Addressable {
    fn size(&self) -> usize;
    fn backup_type(&self) -> BackupType;
    fn backing_storage(&self) -> Vec<u8>;
}
