use crate::cartridge::storage::BackupType;
use crate::cartridge::StorageChip;
use crate::memory::device::Addressable;
use log::error;

const EEPROM_4K_SIZE: u32 = 0x200; // 512 bytes
const EEPROM_64K_SIZE: u32 = 0x10000; // 64 KiB

#[derive(PartialEq, Eq)]
pub enum EepromState {
    Idle,
    ReadCmd,
    WriteCmd,
}

pub struct Eeprom {
    eeprom: Vec<u8>,
    backup_type: BackupType,
    boundary: u32,
    opcode_latch: Option<u8>,
    state: EepromState,
    current_bits_addr: Vec<u8>,
    current_bits_data: Vec<u8>,
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
            opcode_latch: None,
            state: EepromState::Idle,
            current_bits_addr: Vec::with_capacity(14),
            current_bits_data: Vec::with_capacity(64),
        }
    }
}

impl Addressable for Eeprom {
    fn read(&self, _addr: u32) -> u8 {
        //println!("Reading from EEPROM at address: {:08x}", addr);
        0xFF
    }

    fn write(&mut self, _: u32, value: u8) {
        let addr_bits = if self.backup_type == BackupType::Eeprom4k {
            6
        } else {
            14
        };

        // are we idling?
        if self.state == EepromState::Idle
            && let Some(previous_opcode_bit) = self.opcode_latch
        {
            match (previous_opcode_bit, value) {
                (1, 0) => {
                    self.state = EepromState::WriteCmd;
                    println!("EEPROM Write Command");
                }
                (1, 1) => {
                    self.state = EepromState::ReadCmd;
                    println!("EEPROM Read Command");
                    //self.state = EepromState::Idle;
                }
                _ => {
                    error!(
                        "Invalid EEPROM state: previous_opcode_bit = {:08b}, value = {}",
                        previous_opcode_bit, value
                    );
                }
            }

            self.opcode_latch = None;
        } else {
            self.opcode_latch = Some(value);
        }

        // is there a command in progress?
        if self.state == EepromState::WriteCmd {
            if self.current_bits_addr.len() < addr_bits {
                self.current_bits_addr.push(value & 1);
            } else if self.current_bits_data.len() < 64 {
                self.current_bits_data.push(value & 1);
            } else if value & 1 == 0 {
                // We have received a full command
                let addr = ((self.current_bits_addr[0] as u32) << 13)
                    | ((self.current_bits_addr[1] as u32) << 12)
                    | ((self.current_bits_addr[2] as u32) << 11)
                    | ((self.current_bits_addr[3] as u32) << 10)
                    | ((self.current_bits_addr[4] as u32) << 9)
                    | ((self.current_bits_addr[5] as u32) << 8)
                    | ((self.current_bits_addr[6] as u32) << 7)
                    | ((self.current_bits_addr[7] as u32) << 6)
                    | ((self.current_bits_addr[8] as u32) << 5)
                    | ((self.current_bits_addr[9] as u32) << 4)
                    | ((self.current_bits_addr[10] as u32) << 3)
                    | ((self.current_bits_addr[11] as u32) << 2)
                    | ((self.current_bits_addr[12] as u32) << 1)
                    | (self.current_bits_addr[13] as u32);
                let data = {
                    let mut value: u64 = 0;
                    for (i, bit) in self.current_bits_data.iter().enumerate() {
                        value |= (*bit as u64) << (63 - i); // now the full 0-63 range is valid
                    }
                    value
                };

                if addr < self.boundary {
                    println!("EEPROM Write Command: Address = {:08x}, Data = {:08x}", addr, data);
                    let byte_base = (addr as usize) * 8;
                    if byte_base + 8 <= self.eeprom.len() {
                        for i in 0..8 {
                            self.eeprom[byte_base + i] = ((data >> (8 * (7 - i))) & 0xFF) as u8;
                        }
                    }
                } else {
                    println!("EEPROM write address out of bounds: {:08x}", addr);
                }

                self.state = EepromState::Idle;
                self.current_bits_addr.clear();
                self.current_bits_data.clear();
            }
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
}
