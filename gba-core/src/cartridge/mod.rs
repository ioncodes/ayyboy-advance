use crate::memory::device::Addressable;

pub mod database;
pub mod flash;
pub mod sram;
pub mod storage;

pub trait StorageChip: Addressable {
    fn size(&self) -> usize;

    fn storage(&self) -> Vec<u8> {
        let mut storage = vec![0; self.size()];
        for i in 0..self.size() {
            storage[i] = self.read((i as u32) + 0x0E000000);
        }
        storage
    }
}
