use core::panic;

use super::device::{Addressable, IoRegister};
use super::dma::Dma;
use crate::arm7tdmi::decoder::TransferLength;
use crate::arm7tdmi::timer::Timers;
use crate::audio::apu::Apu;
use crate::input::joypad::Joypad;
use crate::memory::registers::{AddrControl, DmaTrigger, Interrupt};
use crate::video::ppu::{Ppu, PpuEvent};
use crate::video::registers::DispStat;
use log::*;

const EWRAM_SIZE: u32 = 0x40000; // 256 KiB
const IWRAM_SIZE: u32 = 0x8000; // 32 KiB
const PALETTE_SIZE: u32 = 0x400; // 1 KiB
const VRAM_SIZE: u32 = 0x20000; // 128 KiB. // VRAM is 96 KiB, but it mirrors every 128 KiB
const OAM_SIZE: u32 = 0x400; // 1 KiB
const SRAM_SIZE: u32 = 0x8000; // 32 KiB

pub struct Mmio {
    pub internal_memory: Box<[u8; 0x04FFFFFF + 1]>,
    pub external_memory: Box<[u8; (0x0FFFFFFF - 0x08000000) + 1]>,
    pub ppu: Ppu,
    pub joypad: Joypad,
    pub apu: Apu,
    pub dma: Dma,
    pub timers: Timers,
    // I/O registers
    pub io_ime: IoRegister,           // IME
    pub io_ie: IoRegister<Interrupt>, // IE
    pub io_if: IoRegister<Interrupt>, // IF
    pub io_halt_cnt: IoRegister<u8>,  // HALTCNT
    // other
    origin_write_length: Option<TransferLength>, // cache this for cases like 8bit VRAM mirrored writes
}

impl Mmio {
    pub fn new() -> Mmio {
        let internal_memory = Box::<[u8; 0x05000000]>::new_zeroed();
        let external_memory = Box::<[u8; 0x08000000]>::new_zeroed();

        Mmio {
            internal_memory: unsafe { internal_memory.assume_init() },
            external_memory: unsafe { external_memory.assume_init() },
            ppu: Ppu::new(),
            joypad: Joypad::new(),
            apu: Apu::new(),
            dma: Dma::new(),
            timers: Timers::new(),
            io_ime: IoRegister::default(),
            io_ie: IoRegister::default(),
            io_if: IoRegister::default(),
            io_halt_cnt: IoRegister(0xff),
            origin_write_length: None,
        }
    }

    pub fn tick_components(&mut self) {
        let events = self.ppu.tick();
        self.timers.tick();

        if events.contains(&PpuEvent::VBlank) && self.ppu.disp_stat.contains_flags(DispStat::VBLANK_IRQ_ENABLE) {
            self.io_if.set_flags(Interrupt::VBLANK);
            trace!("VBLANK interrupt raised");
        }

        if events.contains(&PpuEvent::HBlank) && self.ppu.disp_stat.contains_flags(DispStat::HBLANK_IRQ_ENABLE) {
            self.io_if.set_flags(Interrupt::HBLANK);
            trace!("HBLANK interrupt raised");
        }

        self.transfer_dma(&events);
    }

    pub fn transfer_dma(&mut self, events: &Vec<PpuEvent>) {
        for channel in 0..4 {
            if self.dma.channels[channel].is_enabled()
                && (self.dma.channels[channel].trigger() == DmaTrigger::Immediate
                    || self.dma.channels[channel].trigger() == DmaTrigger::Special // TODO: Special trigger is not implemented, just allow it
                    || (self.dma.channels[channel].trigger() == DmaTrigger::VBlank
                        && events.contains(&PpuEvent::VBlank))
                    || (self.dma.channels[channel].trigger() == DmaTrigger::HBlank
                        && events.contains(&PpuEvent::HBlank)))
            {
                trace!("DMA transfer on channel {}", channel);

                let src = self.dma.channels[channel].src.value();
                let dst = self.dma.channels[channel].dst.value();
                if dst == 0x040000A0 || dst == 0x040000A4 {
                    // TODO: WE SKIP SOUND DMA FOR NOW
                    continue;
                }

                if self.dma.channels[channel].trigger() == DmaTrigger::Special {
                    // Special DMA trigger is not implemented
                    error!(
                        "DMA channel {} triggered with Special trigger, not implemented",
                        channel
                    );
                    continue;
                }

                let size = self.dma.channels[channel].transfer_size();
                let src_ctrl = self.dma.channels[channel].src_addr_control();
                let dst_ctrl = self.dma.channels[channel].dst_addr_control();

                // transfer it at once
                for i in 0..size {
                    let src_addr = match src_ctrl {
                        AddrControl::Increment => src + i as u32,
                        AddrControl::Decrement => src - i as u32,
                        AddrControl::Fixed => src,
                        AddrControl::Reload => unreachable!(),
                    };
                    let dst_addr = match dst_ctrl {
                        AddrControl::Increment => dst + i as u32,
                        AddrControl::Decrement => dst - i as u32,
                        AddrControl::Fixed => dst,
                        AddrControl::Reload => dst + i as u32,
                    };

                    let value = self.read(src_addr);
                    self.write(dst_addr, value);
                }

                // if it's a repeat transfer, we just leave it enabled
                if !self.dma.channels[channel].is_repeat() {
                    self.dma.channels[channel].disable();
                }
            }
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
            0x04000300 => 1, // "After initial reset, the GBA BIOS initializes the register to 01h"
            // Internal and External Memory
            0x00000000..=0x00003FFF => {
                warn!("Reading from BIOS (Open Bus): {:08x}", addr);
                0x69
            }
            0x0400020A..=0x0400020B => self.internal_memory[addr as usize], // Unused
            0x04000000..=0x040003FE => {
                error!("Unmapped I/O read: {:08x}", addr);
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
                let addr = match addr {
                    // Pallete RAM – mirrors every 1 KiB in 0x05000000‑0x050003FF
                    0x05000000..=0x05FFFFFF => 0x05000000 + ((addr - 0x05000000) % PALETTE_SIZE),
                    // VRAM – mirrors every 128 KiB in 0x06000000‑06017FFF (96 KiB)
                    0x06000000..=0x06FFFFFF => 0x06000000 + ((addr - 0x06000000) % VRAM_SIZE),
                    // OAM – mirrors every 1 KiB in 0x07000000‑0x070003FF
                    0x07000000..=0x07FFFFFF => 0x07000000 + ((addr - 0x07000000) % OAM_SIZE),
                    _ => addr,
                };
                self.ppu.read(addr)
            }
            0x08000000..=0x09FFFFFF => self.external_memory[(addr - 0x08000000) as usize],
            0x0A000000..=0x0BFFFFFF => self.external_memory[(addr - 0x0A000000) as usize], // Mirror of 0x08000000..=0x09FFFFFF
            0x0C000000..=0x0DFFFFFF => self.external_memory[(addr - 0x0C000000) as usize], // Mirror of 0x08000000..=0x09FFFFFF
            0x0E000000..=0x0FFFFFFF => {
                // GamePak SRAM – mirrors every 32 KiB in 0x0E000000‑0x0FFFFFFF
                let addr = 0x08000000 + ((addr - 0x0E000000) % SRAM_SIZE);
                self.external_memory[(addr - 0x08000000) as usize]
            }
            _ => {
                error!("Reading from unmapped memory address: {:08x}", addr);
                0x69
            }
        };

        self.origin_write_length = None;

        trace!("Read {:02x} from {:08x}", value, addr);

        value
    }

    pub fn read_u16(&mut self, addr: u32) -> u16 {
        u16::from_le_bytes([self.read(addr), self.read(addr + 1)])
    }

    pub fn read_u32(&mut self, addr: u32) -> u32 {
        u32::from_le_bytes([
            self.read(addr),
            self.read(addr + 1),
            self.read(addr + 2),
            self.read(addr + 3),
        ])
    }

    pub fn write(&mut self, addr: u32, value: u8) {
        trace!("Writing {:02x} to {:08x}", value, addr);

        match addr {
            0x00000000..=0x00003FFF => warn!("Writing to BIOS: {:02x} to {:08x}", value, addr),
            0x04000000..=0x04000056 => self.ppu.write(addr, value), // PPU I/O
            0x04000080..=0x0400008E => self.apu.write(addr, value), // APU I/O
            0x040000B0..=0x040000DF => self.dma.write(addr, value), // DMA I/O
            0x04000100..=0x0400010F => self.timers.write(addr, value), // Timers I/O
            0x04000130..=0x04000133 => self.joypad.write(addr, value), // Joypad I/O
            0x04000200..=0x04000201 => self.io_ie.write(addr, value), // Interrupt Enable
            0x04000202..=0x04000203 => self.io_if.write(addr, value), // Interrupt Flag
            0x04000208..=0x04000209 => self.io_ime.write(addr, value), // Interrupt Master Enable
            0x0400020A..=0x0400020B => self.internal_memory[addr as usize] = value, // Unused
            0x04000301 => self.io_halt_cnt.write(value),            // HALTCNT
            0x04000000..=0x040003FE => {
                error!("Unmapped I/O write: {:02x} to {:08x}", value, addr);
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
                let addr = match addr {
                    // Pallete RAM – mirrors every 1 KiB in 0x05000000‑0x050003FF
                    0x05000000..=0x05FFFFFF => 0x05000000 + ((addr - 0x05000000) % PALETTE_SIZE),
                    // VRAM – mirrors every 96 KiB in 0x06000000‑06017FFF
                    0x06000000..=0x06FFFFFF => 0x06000000 + ((addr - 0x06000000) % VRAM_SIZE),
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
            0x08000000..=0x09FFFFFF => warn!("Writing to GamePak memory: {:02x} to {:08x}", value, addr),
            0x0A000000..=0x0BFFFFFF => warn!("Writing to GamePak memory: {:02x} to {:08x}", value, addr), // Mirror of 0x08000000..=0x09FFFFFF
            0x0C000000..=0x0DFFFFFF => warn!("Writing to GamePak memory: {:02x} to {:08x}", value, addr), // Mirror of 0x08000000..=0x09FFFFFF
            0x0E000000..=0x0FFFFFFF => self.external_memory[(addr - 0x08000000) as usize] = value, // GamePak SRAM
            _ => {
                error!("Writing to unmapped memory address: {:08x}", addr);
            }
        }
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
            _ => panic!("Invalid memory address: {:08x}", addr),
        }
    }
}
