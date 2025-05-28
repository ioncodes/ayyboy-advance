mod emulator;

use emulator::Emulator;
use gba_core::video::{Frame, SCREEN_HEIGHT, SCREEN_WIDTH};
use image::{ImageBuffer, Rgb, RgbImage};
use std::collections::VecDeque;
use std::fs::{self, DirEntry};

fn write_png(frame: &Frame, path: &str) {
    let w = SCREEN_WIDTH as u32;
    let h = SCREEN_HEIGHT as u32;

    let img: RgbImage = ImageBuffer::from_fn(w, h, |x, y| {
        let (r, g, b) = frame[y as usize][x as usize];
        Rgb([r, g, b])
    });

    img.save(path).unwrap()
}

fn extract_files_from_path(path: &str) -> Vec<DirEntry> {
    fs::read_dir(path)
        .expect("Failed to read ROM directory")
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .path()
                .extension()
                .map_or(false, |ext| ext == "zip" || ext == "gba")
        })
        .collect::<Vec<_>>()
}

fn extract_file_stem(entry: &DirEntry) -> String {
    entry
        .path()
        .file_stem()
        .map(|s| s.to_string_lossy())
        .unwrap_or_else(|| "unknown".into())
        .to_string()
}

fn emulate_rom(rom_path: String, output_path: String, filename: String) {
    fs::create_dir_all(&output_path).expect("Failed to create output directory");

    let mut emulator = Emulator::new(rom_path);

    for i in 0usize..500_000 {
        if let Some(frame) = emulator.run_to_frame() {
            if i % 50_000 == 0 {
                let image_path = format!("{}/{}_{}.png", output_path, filename, i);
                write_png(&frame, &image_path);
            }
        } else {
            break;
        }
    }
}

fn main() {
    let rom_folder = std::env::args().nth(1).unwrap_or_else(|| {
        println!("Usage: rom-db <rom_folder>");
        std::process::exit(1);
    });

    const OUTPUT_FOLDER: &str = "rom-db/screenshots";
    const MAX_THREADS: usize = 45;

    fs::create_dir_all(OUTPUT_FOLDER).expect("Failed to create output directory");

    let files = extract_files_from_path(&rom_folder);
    let mut handles = Vec::new();
    let mut active_threads = 0;
    let mut queue = VecDeque::new();

    for file in files {
        let filepath = file.path();
        let filestem = extract_file_stem(&file);

        queue.push_back((
            filepath.to_string_lossy().to_string(),
            format!("{}/{}", OUTPUT_FOLDER, filestem),
            filestem,
        ));
    }

    while !queue.is_empty() || active_threads > 0 {
        while active_threads < MAX_THREADS && !queue.is_empty() {
            if let Some((rom_path, output_path, filename)) = queue.pop_front() {
                let handle = std::thread::spawn(move || {
                    emulate_rom(rom_path, output_path, filename);
                });
                handles.push(handle);
                active_threads += 1;
            }
        }

        if !handles.is_empty() {
            let mut i = 0;
            while i < handles.len() {
                if handles[i].is_finished() {
                    handles.swap_remove(i);
                    active_threads -= 1;
                } else {
                    i += 1;
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    for handle in handles {
        handle.join().unwrap();
    }
}
