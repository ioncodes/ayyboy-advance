#![feature(new_uninit)]

mod arm7tdmi;
mod memory;

use arm7tdmi::cpu::Cpu;
use memory::mmio::Mmio;

const ARM_TEST: &[u8] = include_bytes!("../external/gba_bios.bin");

fn main() {
    let mut mmio = Mmio::new();
    let mut cpu = Cpu::new();

    mmio.load(0, ARM_TEST);

    loop {
        cpu.tick(&mut mmio);
    }
}
