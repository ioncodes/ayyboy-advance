mod emulator;

use crossbeam_channel::{Receiver, Sender};
use emulator::Emulator;
use gba_core::video::{Frame, SCREEN_HEIGHT, SCREEN_WIDTH};
use image::{ImageBuffer, Rgb, RgbImage};

fn write_png(frame: &Frame, path: &str) {
    let w = SCREEN_WIDTH as u32;
    let h = SCREEN_HEIGHT as u32;

    let img: RgbImage = ImageBuffer::from_fn(w, h, |x, y| {
        let (r, g, b) = frame[y as usize][x as usize];
        Rgb([r, g, b])
    });

    img.save(path).unwrap()
}

fn main() {
    let (display_tx, display_rx): (Sender<Frame>, Receiver<Frame>) = crossbeam_channel::bounded(1);

    std::thread::spawn(move || {
        while let Ok(frame) = display_rx.recv() {
            write_png(&frame, "output.png");
        }
    });

    let mut emulator = Emulator::new(
        display_tx,
        "\\\\VIBRATOR\\Roms\\Nintendo - Game Boy Advance\\Asterix & Obelix XXL (Europe) (En,Fr,De,Es,It,Nl).zip"
            .to_string(),
    );

    let handle = std::thread::spawn(move || {
        emulator.run();
    });

    let timeout = std::time::Duration::from_secs(5);
    match handle.join_timeout(timeout) {
        Ok(_) => println!("Emulator has finished running."),
        Err(_) => println!("Emulator timed out after 5 seconds."),
    }
}

trait JoinTimeoutExt {
    fn join_timeout(self, dur: std::time::Duration) -> Result<(), ()>;
}

impl JoinTimeoutExt for std::thread::JoinHandle<()> {
    fn join_timeout(self, dur: std::time::Duration) -> Result<(), ()> {
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::thread;
        let finished = Arc::new(AtomicBool::new(false));
        let finished2 = finished.clone();
        thread::spawn(move || {
            self.join().ok();
            finished2.store(true, Ordering::SeqCst);
        });
        let start = std::time::Instant::now();
        while start.elapsed() < dur {
            if finished.load(Ordering::SeqCst) {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        Err(())
    }
}
