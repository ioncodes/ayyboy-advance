#![feature(new_uninit)]

mod arm7tdmi;
mod memory;
mod video;

use arm7tdmi::cpu::{Cpu, ProcessorMode};
use memory::mmio::Mmio;

//const ARM_TEST: &[u8] = include_bytes!("../external/gba-tests/arm/arm.gba");
const ARM_TEST: &[u8] = include_bytes!("../external/gba-div-test/out/rom.gba");
const BIOS: &[u8] = include_bytes!("../external/gba_bios.bin");

fn main() {
    env_logger::builder().format_timestamp(None).init();

    let mut mmio = Mmio::new();
    mmio.load(0x00000000, BIOS); // bios addr
    mmio.load(0x08000000, ARM_TEST); // gamepak addr

    let mut cpu = Cpu::new();
    cpu.registers.r[13] = 0x03007f00; // sp
    cpu.registers.r[15] = 0x08000000; // pc
    cpu.set_processor_mode(ProcessorMode::User);

    loop {
        cpu.tick(&mut mmio);
        mmio.tick_components();
    }
}
