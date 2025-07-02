const SANYO_MANUFACTURER_ID: u8 = 0x62;
const SANYO_DEVICE_ID: u8 = 0x13;
const PANASONIC_MANUFACTURER_ID: u8 = 0x32;
const PANASONIC_DEVICE_ID: u8 = 0x1B;

// TODO: Switch to https://docs.google.com/spreadsheets/d/16-a3qDDkJJNpaYOEXi-xgTv-j1QznXHt9rTUJNFshjo/edit?pli=1&gid=0#gid=0 maybe?

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BackupType {
    Eeprom4k,
    Eeprom64k,
    Flash512k,
    Flash1m,
    Sram,
    None,
}

impl BackupType {
    pub fn has_rtc(&self) -> bool {
        matches!(self, BackupType::Flash512k | BackupType::Flash1m)
    }

    pub fn manufacturer_id(&self) -> u8 {
        match self {
            BackupType::Flash512k => PANASONIC_MANUFACTURER_ID,
            BackupType::Flash1m => SANYO_MANUFACTURER_ID,
            _ => unreachable!(),
        }
    }

    pub fn device_id(&self) -> u8 {
        match self {
            BackupType::Flash512k => PANASONIC_DEVICE_ID,
            BackupType::Flash1m => SANYO_DEVICE_ID,
            _ => unreachable!(),
        }
    }
}

impl From<u8> for BackupType {
    fn from(value: u8) -> Self {
        match value {
            0 => BackupType::None,
            1 => BackupType::Eeprom4k,
            2 => BackupType::Eeprom64k,
            3 => BackupType::Sram,
            4 => BackupType::Flash512k,
            5 => BackupType::Flash1m,
            _ => unreachable!(),
        }
    }
}

impl std::fmt::Display for BackupType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackupType::Eeprom4k => write!(f, "EEPROM 4K"),
            BackupType::Eeprom64k => write!(f, "EEPROM 64K"),
            BackupType::Flash512k => write!(f, "Flash 512K"),
            BackupType::Flash1m => write!(f, "Flash 1M"),
            BackupType::Sram => write!(f, "SRAM"),
            BackupType::None => write!(f, "None"),
        }?;
        write!(f, " (has RTC: {})", self.has_rtc())
    }
}
