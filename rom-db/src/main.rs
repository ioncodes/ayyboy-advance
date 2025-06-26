mod emulator;

use emulator::Emulator;
use gba_core::input::registers::KeyInput;
use gba_core::video::{Frame, Pixel, SCREEN_HEIGHT, SCREEN_WIDTH};
use image::{ImageBuffer, Rgb, RgbImage};

fn write_png(frame: &Frame, path: &str) {
    let w = SCREEN_WIDTH as u32;
    let h = SCREEN_HEIGHT as u32;

    let img: RgbImage = ImageBuffer::from_fn(w, h, |x, y| match frame[y as usize][x as usize] {
        Pixel::Transparent => Rgb([0, 0, 0]),
        Pixel::Rgb(r, g, b) => Rgb([r, g, b]),
    });

    img.save(path).unwrap()
}

fn emulate_rom(rom_path: String, output_path: String) {
    std::fs::create_dir_all(&output_path).expect("Failed to create output directory");

    let mut emulator = Emulator::new(rom_path);

    for i in 0usize..500_000 {
        if let Some(frame) = emulator.run_to_frame() {
            if i % 100_000 == 0 {
                emulator
                    .cpu
                    .mmio
                    .joypad
                    .set_key_state(KeyInput::A, !emulator.cpu.mmio.joypad.is_key_pressed(KeyInput::A));
                emulator.cpu.mmio.joypad.set_key_state(
                    KeyInput::START,
                    !emulator.cpu.mmio.joypad.is_key_pressed(KeyInput::START),
                );
            }

            if i % 20_000 == 0 {
                let image_path = format!("{}/{}.png", output_path, i);
                write_png(&frame, &image_path);
            }
        } else {
            break;
        }
    }
}

fn main() {
    let rom_path = std::env::args().nth(1).unwrap_or_else(|| {
        println!("Usage: rom-db <rom_path>");
        std::process::exit(1);
    });

    const OUTPUT_FOLDER: &str = "rom-db-ui/public/screenshots";
    std::fs::create_dir_all(OUTPUT_FOLDER).expect("Failed to create output directory");

    let rom_path = std::fs::canonicalize(rom_path).expect("Failed to canonicalize ROM path");
    let rom_name = rom_path.file_stem().unwrap_or_default();
    let output_path = format!("{}/{}", OUTPUT_FOLDER, rom_name.to_string_lossy());

    emulate_rom(rom_path.to_string_lossy().to_string(), output_path);
}
