pub enum BackupType {
    Eeprom4k,
    Eeprom64k,
    Flash512k { rtc: bool, chip_id: u16 },
    Flash1m { rtc: bool, chip_id: u16 },
    Sram,
    None,
}

impl BackupType {
    pub fn has_rtc(&self) -> bool {
        matches!(
            self,
            BackupType::Flash512k { rtc: true, .. } | BackupType::Flash1m { rtc: true, .. }
        )
    }

    pub fn manufacturer(&self) -> &'static str {
        match self {
            BackupType::Flash512k { chip_id, .. } => match *chip_id {
                0x3D1F => "Atmel",
                0xD4BF => "SST",
                0x1B32 => "Panasonic",
                _ => unreachable!(),
            },
            BackupType::Flash1m { chip_id, .. } => match *chip_id {
                0x09C2 => "Macronix",
                0x1362 => "Sanyo",
                _ => unreachable!(),
            },
            _ => unreachable!(),
        }
    }
}

impl From<u8> for BackupType {
    fn from(value: u8) -> Self {
        match value {
            0x00 => BackupType::Eeprom4k,
            0x01 => BackupType::Eeprom4k,
            0x02 => BackupType::Eeprom64k,
            0x03 => BackupType::Eeprom64k,
            0x04 => BackupType::Flash512k {
                rtc: true,
                chip_id: 0x3D1F,
            },
            0x05 => BackupType::Flash512k {
                rtc: false,
                chip_id: 0x3D1F,
            },
            0x06 => BackupType::Flash512k {
                rtc: true,
                chip_id: 0xD4BF,
            },
            0x07 => BackupType::Flash512k {
                rtc: false,
                chip_id: 0xD4BF,
            },
            0x08 => BackupType::Flash512k {
                rtc: true,
                chip_id: 0x1B32,
            },
            0x09 => BackupType::Flash512k {
                rtc: false,
                chip_id: 0x1B32,
            },
            0x0A => BackupType::Flash1m {
                rtc: true,
                chip_id: 0x09C2,
            },
            0x0B => BackupType::Flash1m {
                rtc: false,
                chip_id: 0x09C2,
            },
            0x0C => BackupType::Flash1m {
                rtc: true,
                chip_id: 0x1362,
            },
            0x0D => BackupType::Flash1m {
                rtc: false,
                chip_id: 0x1362,
            },
            0x0E => BackupType::Sram,
            0x0F => BackupType::None,
            _ => unreachable!(),
        }
    }
}

impl std::fmt::Display for BackupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupType::Eeprom4k => write!(f, "EEPROM 4k"),
            BackupType::Eeprom64k => write!(f, "EEPROM 64k"),
            BackupType::Flash512k { rtc, chip_id } => {
                if *rtc {
                    write!(f, "Flash 512K with RTC ({:04X} ID)", chip_id)
                } else {
                    write!(f, "Flash 512K without RTC ({:04X} ID)", chip_id)
                }
            }
            BackupType::Flash1m { rtc, chip_id } => {
                if *rtc {
                    write!(f, "Flash 1M with RTC ({:04X} ID)", chip_id)
                } else {
                    write!(f, "Flash 1M without RTC ({:04X} ID)", chip_id)
                }
            }
            BackupType::Sram => write!(f, "SRAM/FRAM"),
            BackupType::None => write!(f, "No backup"),
        }
    }
}
