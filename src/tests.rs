#[cfg(test)]
mod tests {
    use crate::arm7tdmi::cpu::{Cpu, ProcessorMode};
    use crate::arm7tdmi::decoder::Instruction;
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

        let mut trace: Vec<(u32, Instruction)> = Vec::new();

        loop {
            if let Some((instr, state)) = cpu.tick(&mut mmio) {
                trace.push((state.pc, instr));
            }
            mmio.tick_components();

            if cpu.registers.r[15] == 0x08001e18 {
                // arm.gba SWI to extract failed test

                for idx in 0..trace.len() {
                    let idx = trace.len() - idx - 1;
                    let (pc, _) = &trace[idx];

                    // find the m_exit handler
                    if *pc != 0x08001d4c {
                        continue;
                    }

                    // walk back the trace
                    for faulting_idx in 0..20 {
                        let (faulting_pc, faulting_instr) = &trace[idx - faulting_idx];
                        println!("{:08x}: {}", faulting_pc, faulting_instr);
                    }
                }

                assert!(false, "Failed test: {}", cpu.registers.r[12]);
            }

            if cpu.registers.r[15] == 0x08001d8c {
                break; // reached "all tests passed"
            }
        }

        assert!(true, "All tests passed");
    }
}
