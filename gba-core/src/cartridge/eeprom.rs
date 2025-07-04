use crate::cartridge::StorageChip;
use crate::cartridge::storage::BackupType;
use crate::memory::device::{Addressable, Saveable};
use std::cell::{Cell, RefCell};
use tracing::debug;

const EEPROM_4K_SIZE: u32 = 0x200; // 512 bytes
const EEPROM_64K_SIZE: u32 = 0x10000; // 64 KiB

#[derive(Default, Clone, Copy)]
enum EepromState {
    #[default]
    Idle,
    Command {
        first_bit: u8,
    },
    WriteAddress {
        addr: u32,
        bits_left: u8,
    },
    WriteData {
        addr: u32,
        data: u64,
        bits_left: u8,
    },
    WriteFinalize {
        addr: u32,
        data: u64,
    },
    ReadAddress {
        addr: u32,
        bits_left: u8,
    },
    ReadTransfer {
        addr: u32,
        bits_left: u8,
    },
}

pub struct Eeprom {
    pub eeprom: Vec<u8>,
    pub backup_type: BackupType,
    boundary: u32,
    state: RefCell<EepromState>,
    last_read_bit: Cell<u8>,
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
            state: RefCell::new(EepromState::Idle),
            last_read_bit: Cell::new(1),
        }
    }
}

impl Addressable for Eeprom {
    fn read(&self, addr: u32) -> u8 {
        if addr & 1 == 1 {
            return self.last_read_bit.get();
        }

        let mut state = self.state.borrow_mut();

        let bit = match *state {
            EepromState::ReadTransfer {
                addr,
                ref mut bits_left,
            } => {
                if *bits_left == 0 {
                    // If no bits left, return the last read bit and reset state
                    *state = EepromState::Idle;
                    self.last_read_bit.get()
                } else {
                    *bits_left -= 1;
                    if *bits_left >= 64 {
                        0
                    } else {
                        let step = 63 - *bits_left;
                        let start = (addr as usize) * 8;

                        // reverse the byte order so that LE storage is streamed MSB‐first
                        let byte_index = start + (7 - ((step as usize) >> 3));
                        let bit_index = 7 - (step & 7);

                        let value = if byte_index < self.eeprom.len() {
                            (self.eeprom[byte_index] >> bit_index) & 1
                        } else {
                            0
                        };

                        if *bits_left == 0 {
                            *state = EepromState::Idle;
                        }
                        value
                    }
                }
            }
            _ => 1,
        };

        self.last_read_bit.set(bit);

        bit
    }

    fn write(&mut self, addr: u32, value: u8) {
        if addr & 1 == 1 {
            return;
        }

        let bit = value & 1;
        let mut state = self.state.borrow_mut();

        match *state {
            EepromState::Idle => {
                // Latch first bit of command
                *state = EepromState::Command { first_bit: bit };
            }
            EepromState::Command { first_bit } => {
                let command = (first_bit << 1) | bit;
                let addr_bit_size = if self.backup_type == BackupType::Eeprom4k {
                    6
                } else {
                    14
                };

                match command {
                    0b10 => {
                        // WRITE command
                        *state = EepromState::WriteAddress {
                            addr: 0,
                            bits_left: addr_bit_size,
                        }
                    }
                    0b11 => {
                        // READ command
                        *state = EepromState::ReadAddress {
                            addr: 0,
                            bits_left: addr_bit_size,
                        }
                    }
                    _ => *state = EepromState::Idle,
                }
            }
            EepromState::WriteAddress {
                ref mut addr,
                ref mut bits_left,
            } => {
                *addr = (*addr << 1) | bit as u32; // add the bit to the address
                *bits_left -= 1;

                if *bits_left == 0 {
                    // All bits of the address have been received
                    // Start fetching data bits
                    *state = EepromState::WriteData {
                        addr: *addr,
                        data: 0,
                        bits_left: 64,
                    };
                }
            }
            EepromState::WriteData {
                addr,
                ref mut data,
                ref mut bits_left,
            } => {
                *data = (*data << 1) | bit as u64; // add the bit to the data
                *bits_left -= 1;

                if *bits_left == 0 {
                    // All bits of the data have been received
                    // Prepare to finalize the write
                    *state = EepromState::WriteFinalize { addr, data: *data };
                }
            }
            EepromState::WriteFinalize { addr, data } => {
                // STOP bit
                if bit == 0 {
                    let start = (addr as usize) * 8;
                    // write in little-endian order
                    let bytes = data.to_le_bytes();

                    if start + bytes.len() <= self.eeprom.len() {
                        debug!(target: "storage", "Writing to EEPROM at address: {:08X}, data: {:02x?}", start, bytes);
                        self.eeprom[start..start + bytes.len()].copy_from_slice(&bytes);
                    }
                }

                *state = EepromState::Idle;
            }
            EepromState::ReadAddress {
                ref mut addr,
                ref mut bits_left,
            } => {
                *addr = (*addr << 1) | bit as u32; // Add the bit to the address
                *bits_left -= 1;

                if *bits_left == 0 {
                    // All bits of the address have been received
                    // Start the read transfer
                    *state = EepromState::ReadTransfer {
                        addr: *addr,
                        bits_left: 68,
                    };
                }
            }
            EepromState::ReadTransfer { .. } => {} // Not used once READ command is initiated
        }
    }
}

impl StorageChip for Eeprom {
    fn size(&self) -> usize {
        self.boundary as usize
    }

    fn backup_type(&self) -> BackupType {
        self.backup_type
    }

    fn backing_storage(&self) -> Vec<u8> {
        self.eeprom.clone()
    }
}

impl Saveable for Eeprom {
    fn aggregate_storage(&self) -> Vec<u8> {
        self.eeprom.clone()
    }

    fn load_storage(&mut self, data: &[u8]) {
        if data.len() != self.eeprom.len() {
            panic!(
                "Invalid EEPROM data length: expected {}, got {}",
                self.eeprom.len(),
                data.len()
            );
        }

        self.eeprom.copy_from_slice(data);
        debug!(target: "storage", "EEPROM loaded with {} bytes", data.len());
    }
}
