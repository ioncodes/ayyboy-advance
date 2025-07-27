use core::panic;

use super::device::{Addressable, IoRegister};
use super::dma::Dma;
use crate::arm7tdmi::decoder::TransferLength;
use crate::arm7tdmi::timer::Timers;
use crate::audio::apu::Apu;
use crate::cartridge::StorageChip;
use crate::cartridge::eeprom::Eeprom;
use crate::cartridge::flash::Flash;
use crate::cartridge::sram::Sram;
use crate::cartridge::storage::BackupType;
use crate::input::joypad::Joypad;
use crate::memory::registers::{AddrControl, DmaTrigger, Interrupt};
use crate::video::ppu::{Ppu, PpuEvent};
use crate::video::registers::DispStat;
use tracing::*;

const EWRAM_SIZE: u32 = 0x40000; // 256 KiB
const IWRAM_SIZE: u32 = 0x8000; // 32 KiB
const PALETTE_SIZE: u32 = 0x400; // 1 KiB
const VRAM_PHYS_SIZE: u32 = 0x18000;
const VRAM_WINDOW_SIZE: u32 = 0x20000; // 128 KiB
const OAM_SIZE: u32 = 0x400; // 1 KiB

pub struct Mmio {
    pub internal_memory: Box<[u8; 0x04FFFFFF + 1]>,
    pub external_memory: Box<[u8; (0x0DFFFFFF - 0x08000000) + 1]>,
    pub ppu: Ppu,
    pub joypad: Joypad,
    pub apu: Apu,
    pub dma: Dma,
    pub timers: Timers,
    pub storage_chip: Box<dyn StorageChip>, // Storage chip, e.g. SRAM, EEPROM, Flash
    // I/O registers
    pub io_ime: IoRegister,           // IME
    pub io_ie: IoRegister<Interrupt>, // IE
    pub io_if: IoRegister<Interrupt>, // IF
    pub io_halt_cnt: IoRegister<u8>,  // HALTCNT
    pub io_postflg: IoRegister<u8>,   // POSTFLG
    // other
    pub last_rw_addr: Vec<u32>,                      // track the last read/write addresses
    pub origin_write_length: Option<TransferLength>, // cache this for cases like 8bit VRAM mirrored writes
    pub executing_bios: bool,
    pub openbus_bios: u32,
}

impl Mmio {
    pub fn new(backup_type: BackupType, has_rtc: bool) -> Mmio {
        let internal_memory = Box::<[u8; 0x05000000]>::new_zeroed();
        let external_memory = Box::<[u8; 0x06000000]>::new_zeroed();

        let storage_chip: Box<dyn StorageChip> = match backup_type {
            BackupType::Sram => Box::new(Sram::new()),
            BackupType::Flash512k | BackupType::Flash1m => Box::new(Flash::new(backup_type.clone(), has_rtc)),
            BackupType::Eeprom4k | BackupType::Eeprom64k => Box::new(Eeprom::new(backup_type.clone())),
            _ => {
                error!(target: "mmio", "Unsupported backup type: {}, defaulting to SRAM", backup_type);
                Box::new(Sram::new())
            }
        };

        Mmio {
            internal_memory: unsafe { internal_memory.assume_init() },
            external_memory: unsafe { external_memory.assume_init() },
            ppu: Ppu::new(),
            joypad: Joypad::new(),
            apu: Apu::new(),
            dma: Dma::new(),
            timers: Timers::new(),
            storage_chip,
            io_ime: IoRegister::default(),
            io_ie: IoRegister::default(),
            io_if: IoRegister::default(),
            io_halt_cnt: IoRegister(0xff),
            io_postflg: IoRegister::default(),
            origin_write_length: None,
            last_rw_addr: Vec::new(), // initialize last_rw_addr to zero
            executing_bios: true,
            openbus_bios: 0,
        }
    }

    pub fn tick_components(&mut self) {
        let events = self.ppu.tick();
        self.timers.tick();

        if events.contains(&PpuEvent::VBlank) && self.ppu.disp_stat.contains_flags(DispStat::VBLANK_IRQ_ENABLE) {
            self.io_if.set_flags(Interrupt::VBLANK);
            trace!(target: "irq", "VBLANK interrupt raised");
        }

        if events.contains(&PpuEvent::HBlank) && self.ppu.disp_stat.contains_flags(DispStat::HBLANK_IRQ_ENABLE) {
            self.io_if.set_flags(Interrupt::HBLANK);
            trace!(target: "irq", "HBLANK interrupt raised");
        }

        self.process_dma_channels(&events);
    }

    pub fn process_dma_channels(&mut self, events: &Vec<PpuEvent>) {
        for channel_id in 0..4 {
            if !self.dma.channels[channel_id].is_enabled() {
                continue;
            }

            let is_immediate_trigger = self.dma.channels[channel_id].trigger() == DmaTrigger::Immediate;
            let is_special_trigger = self.dma.channels[channel_id].trigger() == DmaTrigger::Special;
            let is_vblank_trigger =
                self.dma.channels[channel_id].trigger() == DmaTrigger::VBlank && events.contains(&PpuEvent::VBlank);
            let is_hblank_trigger =
                self.dma.channels[channel_id].trigger() == DmaTrigger::HBlank && events.contains(&PpuEvent::HBlank);

            if !is_immediate_trigger && !is_special_trigger && !is_vblank_trigger && !is_hblank_trigger {
                continue;
            }

            let src = self.dma.channels[channel_id].src.value();
            let dst = self.dma.channels[channel_id].dst.value();

            if dst == 0x040000A0 || dst == 0x040000A4 {
                // TODO: Skip sound DMA for now
                if !self.dma.channels[channel_id].is_repeat() {
                    self.dma.channels[channel_id].disable();
                }

                if self.dma.channels[channel_id].trigger_irq() {
                    let flags = match channel_id {
                        0 => Interrupt::DMA0,
                        1 => Interrupt::DMA1,
                        2 => Interrupt::DMA2,
                        3 => Interrupt::DMA3,
                        _ => unreachable!(),
                    };
                    self.io_if.set_flags(flags);
                    trace!(target: "irq", "DMA{} interrupt raised", channel_id);
                }

                continue;
            }

            debug!(target: "mmio", "DMA transfer on channel {}, src: {:08X}, dst: {:08X}, units: {}, size: {}",
                    channel_id, src, dst,
                    self.dma.channels[channel_id].transfer_units(),
                    self.dma.channels[channel_id].transfer_size());

            self.transfer_dma(channel_id, src, dst);

            // raise interrupt if enabled
            if self.dma.channels[channel_id].trigger_irq() {
                let flags = match channel_id {
                    0 => Interrupt::DMA0,
                    1 => Interrupt::DMA1,
                    2 => Interrupt::DMA2,
                    3 => Interrupt::DMA3,
                    _ => unreachable!(),
                };
                self.io_if.set_flags(flags);
                trace!(target: "irq", "DMA{} interrupt raised", channel_id);
            }
        }
    }

    pub fn transfer_dma(&mut self, channel_id: usize, src: u32, dst: u32) {
        let units = self.dma.channels[channel_id].transfer_units();
        let unit_size = self.dma.channels[channel_id].transfer_size() as u16;
        let src_ctrl = self.dma.channels[channel_id].src_addr_control();
        let dst_ctrl = self.dma.channels[channel_id].dst_addr_control();
        let initial_cnt = self.dma.channels[channel_id].cnt.value();

        // transfer it at once
        for i in 0..units {
            let offset = (i as u32) * unit_size as u32;

            let src_addr = match src_ctrl {
                AddrControl::Increment => src + offset,
                AddrControl::Decrement => src - offset,
                AddrControl::Fixed => src,
                AddrControl::Reload => unreachable!(),
            } & !(unit_size as u32 - 1);
            let dst_addr = match dst_ctrl {
                AddrControl::Increment => dst + offset,
                AddrControl::Decrement => dst - offset,
                AddrControl::Fixed => dst,
                AddrControl::Reload => dst + offset,
            } & !(unit_size as u32 - 1);

            if unit_size == 4 {
                let value = self.read_u32(src_addr);
                self.write_u32(dst_addr, value);
            } else {
                let value = self.read_u16(src_addr);
                self.write_u16(dst_addr, value);
            }
        }

        let final_src = match src_ctrl {
            AddrControl::Increment => src + units as u32 * unit_size as u32,
            AddrControl::Decrement => src - units as u32 * unit_size as u32,
            _ => src,
        };

        let calc_dst = match dst_ctrl {
            AddrControl::Increment => dst + units as u32 * unit_size as u32,
            AddrControl::Decrement => dst - units as u32 * unit_size as u32,
            AddrControl::Fixed | AddrControl::Reload => dst + units as u32 * unit_size as u32,
        };

        let final_dst = if dst_ctrl == AddrControl::Reload { dst } else { calc_dst };

        // update registers
        self.dma.channels[channel_id].src.set(final_src);
        self.dma.channels[channel_id].dst.set(final_dst);

        let cnt = if self.dma.channels[channel_id].is_repeat() {
            initial_cnt
        } else {
            0
        };
        self.dma.channels[channel_id].cnt.set(cnt);

        // if it's a repeat transfer, we just leave it enabled
        if !self.dma.channels[channel_id].is_repeat() {
            self.dma.channels[channel_id].disable();
        }
    }

    pub fn read(&mut self, addr: u32) -> u8 {
        let value = match addr {
            // I/O Registers & Hooks
            0x04000000..=0x04000056 => self.ppu.read(addr),    // PPU I/O
            0x04000080..=0x0400008E => self.apu.read(addr),    // APU I/O
            0x040000B0..=0x040000DF => self.dma.read(addr),    // DMA I/O, 0x40000E0 = unused
            0x04000100..=0x0400010F => self.timers.read(addr), // Timers I/O
            0x04000130..=0x04000133 => self.joypad.read(addr), // Joypad I/O
            0x04000200..=0x04000201 => self.io_ie.read(addr),  // Interrupt Enable
            0x04000202..=0x04000203 => self.io_if.read(addr),  // Interrupt Flag
            0x04000208..=0x04000209 => self.io_ime.read(addr), // Interrupt Master Enable
            0x04000301 => self.io_halt_cnt.read(),             // HALTCNT
            0x04000300 => self.io_postflg.read(), // POSTFLG -> "After initial reset, the GBA BIOS initializes the register to 01h"
            // Internal and External Memory
            0x00000000..=0x00003FFF if self.executing_bios => self.internal_memory[addr as usize],
            0x00000000..=0x00003FFF if !self.executing_bios => {
                // BIOS open bus read
                let shift = ((addr & 3) * 8) as u32;
                let value = ((self.openbus_bios >> shift) & 0xFF) as u8;
                debug!(target: "mmio", "Reading from BIOS open bus: {:08X} => {:02X}", addr, value);
                value
            }
            0x0400020A..=0x0400020B => self.internal_memory[addr as usize], // Unused
            0x04000000..=0x040003FE => {
                error!(target: "mmio", "Unmapped I/O read: {:08X}", addr);
                self.internal_memory[addr as usize]
            }
            0x00000000..=0x04FFFFFF => {
                let addr = match addr {
                    // External WRAM – mirrors every 256 KiB in 0x02000000‑0x02FFFFFF
                    0x02000000..=0x02FFFFFF => 0x02000000 + ((addr - 0x02000000) % EWRAM_SIZE),
                    // Internal WRAM – mirrors every 32 KiB in 0x03000000‑0x03FFFFFF
                    0x03000000..=0x03FFFFFF => 0x03000000 + ((addr - 0x03000000) % IWRAM_SIZE),
                    _ => addr,
                };
                self.internal_memory[addr as usize]
            }
            0x05000000..=0x07FFFFFF => {
                let dispcnt = self.ppu.disp_cnt.value();
                let bg_mode = dispcnt.bg_mode();

                let addr = match addr {
                    // Pallete RAM – mirrors every 1 KiB in 0x05000000‑0x050003FF
                    0x05000000..=0x05FFFFFF => 0x05000000 + ((addr - 0x05000000) % PALETTE_SIZE),
                    // VRAM – 96 KiB + 32 KiB mirror inside each 128 KiB window
                    0x06000000..=0x06FFFFFF => {
                        let mut offset = (addr - 0x0600_0000) % VRAM_WINDOW_SIZE;
                        if offset >= VRAM_PHYS_SIZE {
                            offset -= 0x0080_00;
                        }
                        0x0600_0000 + offset
                    }
                    // Lower 16 KiB of OAM is used for backgrounds in modes 3-5
                    0x07000000..=0x07004000 if bg_mode >= 3 => addr,
                    // OAM – mirrors every 1 KiB in 0x07000000‑0x070003FF
                    0x07000000..=0x07FFFFFF if bg_mode < 3 => 0x07000000 + ((addr - 0x07000000) % OAM_SIZE),
                    _ => addr,
                };
                self.ppu.read(addr)
            }
            0x08000000..=0x09FFFFFF => self.external_memory[(addr - 0x08000000) as usize],
            0x0A000000..=0x0BFFFFFF => self.external_memory[(addr - 0x0A000000) as usize], // Mirror of 0x08000000..=0x09FFFFFF
            0x0D000000..=0x0DFFFFFF
                if matches!(
                    self.storage_chip.backup_type(),
                    BackupType::Eeprom4k | BackupType::Eeprom64k
                ) =>
            {
                // TODO: I think this doesn't handle the EEPROM correctly, but it should be fine for now
                self.storage_chip.read(addr)
            }
            0x0C000000..=0x0DFFFFFF => self.external_memory[(addr - 0x0C000000) as usize], // Mirror of 0x08000000..=0x09FFFFFF
            0x0E000000..=0x0FFFFFFF => self.storage_chip.read(addr),
            _ => {
                error!(target: "mmio", "Reading from unmapped memory address: {:08X}", addr);
                0xFF
            }
        };

        self.origin_write_length = None;
        self.last_rw_addr.push(addr);

        trace!(target: "mmio", "Read {:02X} from {:08X}", value, addr);

        value
    }

    pub fn read_u16(&mut self, addr: u32) -> u16 {
        u16::from_le_bytes([self.read(addr), self.read(addr + 1)])
    }

    pub fn read_u32(&mut self, addr: u32) -> u32 {
        let value = u32::from_le_bytes([
            self.read(addr),
            self.read(addr + 1),
            self.read(addr + 2),
            self.read(addr + 3),
        ]);

        if self.executing_bios && (0x00000000..=0x00003FFF).contains(&addr) {
            self.openbus_bios = value;
        }

        value
    }

    pub fn write(&mut self, addr: u32, value: u8) {
        trace!(target: "mmio", "Writing {:02X} to {:08X}", value, addr);

        match addr {
            0x00000000..=0x00003FFF => debug!(target: "mmio", "Writing to BIOS: {:02X} to {:08X}", value, addr),
            0x04000000..=0x04000056 => self.ppu.write(addr, value), // PPU I/O
            0x04000080..=0x0400008E => self.apu.write(addr, value), // APU I/O
            0x040000B0..=0x040000DF => self.dma.write(addr, value), // DMA I/O
            0x04000100..=0x0400010F => self.timers.write(addr, value), // Timers I/O
            0x04000130..=0x04000133 => self.joypad.write(addr, value), // Joypad I/O
            0x04000200..=0x04000201 => self.io_ie.write(addr, value), // Interrupt Enable
            0x04000202..=0x04000203 => self.io_if.write(addr, value), // Interrupt Flag
            0x04000208..=0x04000209 => self.io_ime.write(addr, value), // Interrupt Master Enable
            0x0400020A..=0x0400020B => self.internal_memory[addr as usize] = value, // Unused
            0x04000300 => self.io_postflg.write(value), // POSTFLG -> "After initial reset, the GBA BIOS initializes the register to 01h"
            0x04000301 => self.io_halt_cnt.write(value), // HALTCNT
            0x04000000..=0x040003FE => {
                error!(target: "mmio", "Unmapped I/O write: {:02X} to {:08X}", value, addr);
                self.internal_memory[addr as usize] = value; // Unmapped I/O region
            }
            0x00000000..=0x04FFFFFF => {
                let addr = match addr {
                    // External WRAM – mirrors every 256 KiB in 0x02000000‑0x02FFFFFF
                    0x02000000..=0x02FFFFFF => 0x02000000 + ((addr - 0x02000000) % EWRAM_SIZE),
                    // Internal WRAM – mirrors every 32 KiB in 0x03000000‑0x03FFFFFF
                    0x03000000..=0x03FFFFFF => 0x03000000 + ((addr - 0x03000000) % IWRAM_SIZE),
                    _ => addr,
                };
                self.internal_memory[addr as usize] = value;
            }
            0x05000000..=0x07FFFFFF => {
                let dispcnt = self.ppu.disp_cnt.value();
                let bg_mode = dispcnt.bg_mode();

                let addr = match addr {
                    // Pallete RAM – mirrors every 1 KiB in 0x05000000‑0x050003FF
                    0x05000000..=0x05FFFFFF => 0x05000000 + ((addr - 0x05000000) % PALETTE_SIZE),
                    // VRAM – 96 KiB + 32 KiB mirror inside each 128 KiB window
                    0x06000000..=0x06FFFFFF => {
                        let mut offset = (addr - 0x0600_0000) % VRAM_WINDOW_SIZE;
                        if offset >= VRAM_PHYS_SIZE {
                            offset -= 0x0080_00;
                        }
                        0x0600_0000 + offset
                    }
                    // Lower 16 KiB of OAM is used for backgrounds in modes 3-5
                    0x07000000..=0x07004000 if bg_mode >= 3 => addr,
                    // OAM – mirrors every 1 KiB in 0x07000000‑0x070003FF
                    0x07000000..=0x07FFFFFF => 0x07000000 + ((addr - 0x07000000) % OAM_SIZE),
                    _ => addr,
                };

                // self.origin_write_length == None implies 8bit write
                match addr {
                    0x06000000..=0x06017FFF if self.origin_write_length == None => {
                        // 8bit write to VRAM mirrors to full halfword
                        let addr = addr & !1; // align to halfword
                        self.ppu.write(addr, value);
                        self.ppu.write(addr + 1, value);
                    }
                    // Atem — 12:06 AM
                    // 8-bit writes to OBJ VRAM and OAM are ignored
                    0x07000000..=0x070003FF if self.origin_write_length == None => {}
                    _ => self.ppu.write(addr, value),
                }
            }
            0x08000000..=0x09FFFFFF => {
                debug!(target: "mmio", "Writing to GamePak memory: {:02X} to {:08X}", value, addr)
            }
            0x0A000000..=0x0BFFFFFF => {
                debug!(target: "mmio", "Writing to GamePak memory: {:02X} to {:08X}", value, addr)
            } // Mirror of 0x08000000..=0x09FFFFFF
            0x0D000000..=0x0DFFFFFF
                if matches!(
                    self.storage_chip.backup_type(),
                    BackupType::Eeprom4k | BackupType::Eeprom64k
                ) =>
            {
                // TODO: I think this doesn't handle the EEPROM correctly, but it should be fine for now
                self.storage_chip.write(addr, value);
            }
            0x0C000000..=0x0DFFFFFF => {
                debug!(target: "mmio", "Writing to GamePak memory: {:02X} to {:08X}", value, addr)
            } // Mirror of 0x08000000..=0x09FFFFFF
            0x0E000000..=0x0FFFFFFF => self.storage_chip.write(addr, value),
            _ => {
                error!(target: "mmio", "Writing to unmapped memory address: {:08X}", addr);
            }
        }

        self.last_rw_addr.push(addr);
    }

    pub fn write_u16(&mut self, addr: u32, value: u16) {
        self.origin_write_length = Some(TransferLength::HalfWord);

        let [a, b] = value.to_le_bytes();
        self.write(addr, a);
        self.write(addr + 1, b);

        self.origin_write_length = None; // reset after writing
    }

    pub fn write_u32(&mut self, addr: u32, value: u32) {
        self.origin_write_length = Some(TransferLength::Word);

        let [a, b, c, d] = value.to_le_bytes();
        self.write(addr, a);
        self.write(addr + 1, b);
        self.write(addr + 2, c);
        self.write(addr + 3, d);

        self.origin_write_length = None; // reset after writing
    }

    pub fn load(&mut self, addr: u32, data: &[u8]) {
        let addr = addr as usize;
        match addr {
            0x00000000..=0x04FFFFFF => self.internal_memory[addr..addr + data.len()].copy_from_slice(data),
            0x05000000..=0x07FFFFFF => self.ppu.load(addr as u32, data),
            0x08000000..=0x0FFFFFFF => {
                self.external_memory[(addr - 0x08000000)..(addr - 0x08000000) + data.len()].copy_from_slice(data)
            }
            _ => panic!("Invalid memory address: {:08X}", addr),
        }
    }

    pub fn enable_bios_access(&mut self) {
        self.executing_bios = true;
    }

    pub fn disable_bios_access(&mut self) {
        self.executing_bios = false;
    }
}
