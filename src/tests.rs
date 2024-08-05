#[cfg(test)]
mod tests {
    use crate::arm7tdmi::cpu::{Cpu, ProcessorMode};
    use crate::memory::mmio::Mmio;

    const BIOS: &[u8] = include_bytes!("../external/gba_bios.bin");
    const ARM_TEST: &[u8] = include_bytes!("../external/gba-tests/arm/arm.gba");

    #[test]
    fn run_arm_gba() {
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

            if cpu.registers.r[15] == 0x08001e18 {
                // arm.gba SWI to extract failed test
                assert!(false, "Failed test: {}", cpu.registers.r[12]);
            }

            if cpu.registers.r[15] == 0x08001d8c {
                break; // reached "all tests passed"
            }
        }

        assert!(true, "All tests passed");
    }
}
