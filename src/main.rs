#![feature(new_uninit)]

mod arm7tdmi;
mod memory;

use arm7tdmi::cpu::Cpu;
use memory::mmio::Mmio;

const ARM_TEST: &[u8] = include_bytes!("../external/gba_bios.bin");

fn main() {
    let mut mmio = Mmio::new();
    mmio.load(0, ARM_TEST);

    let mut cpu = Cpu::new();

    loop {
        cpu.tick(&mut mmio);
    }
}
