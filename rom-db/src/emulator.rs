use gba_core::arm7tdmi::error::CpuError;
use gba_core::gba::Gba;
use gba_core::video::Frame;
use std::fs::File;
use std::io::{Cursor, Read};
use zip::ZipArchive;

pub struct Emulator {
    pub gba: Gba,
    frame_rendered: bool,
}

impl Emulator {
    pub fn new(rom_path: String) -> Self {
        // Load ROM from file
        let mut rom_data = Vec::new();
        let mut rom_file = File::open(&rom_path).expect("Failed to open ROM file");
        rom_file.read_to_end(&mut rom_data).expect("Failed to read ROM file");

        // If it's a ZIP file, extract the ROM
        if rom_path.ends_with(".zip") {
            rom_data = Self::unzip_archive(&rom_data);
        }

        let mut gba = Gba::new(&rom_data, &[]);
        gba.cpu.skip_bios();

        Self {
            gba,
            frame_rendered: false,
        }
    }

    pub fn run_to_frame(&mut self) -> Option<Frame> {
        let mut i = 0;
        loop {
            if i > 100_000_000 {
                // bail in case smth goes wrong
                println!("Emulation took too long, bailing.");
                return None;
            }

            i += 1;

            match self.gba.cpu.tick() {
                Err(CpuError::FailedToDecode) => return None,
                _ => {}
            }
            self.gba.cpu.mmio.tick_components();

            if self.gba.cpu.mmio.ppu.scanline.0 == 160 && !self.frame_rendered {
                self.frame_rendered = true;
                return Some(self.gba.cpu.mmio.ppu.get_frame());
            } else if self.gba.cpu.mmio.ppu.scanline.0 == 0 && self.frame_rendered {
                self.frame_rendered = false;
            }
        }
    }

    fn unzip_archive(buffer: &[u8]) -> Vec<u8> {
        let mut archive = ZipArchive::new(Cursor::new(buffer)).unwrap();

        let gba_index = (0..archive.len())
            .filter(|&i| archive.by_index(i).unwrap().name().contains(".gba"))
            .next()
            .unwrap_or_else(|| panic!("No .gba file found in archive"));

        let mut file = archive.by_index(gba_index).unwrap();
        let mut buffer = Vec::with_capacity(file.size() as usize);
        let _ = file.read_to_end(&mut buffer).unwrap();

        buffer
    }
}
