#[cfg(test)]
mod tests {
    use crate::arm7tdmi::cpu::Cpu;
    use crate::arm7tdmi::decoder::{Instruction, Register};
    use crate::arm7tdmi::mode::ProcessorMode;
    use crate::cartridge::storage::BackupType;
    use crate::memory::mmio::Mmio;

    const BIOS: &[u8] = include_bytes!("../../external/gba_bios.bin");
    const ARM_TEST: &[u8] = include_bytes!("../../external/gba-tests/arm/arm.gba");

    #[test]
    fn run_arm_gba() {
        let mut mmio = Mmio::new(BackupType::Sram, false);
        mmio.load(0x00000000, BIOS); // bios addr
        mmio.load(0x08000000, ARM_TEST); // gamepak addr

        let mut cpu = Cpu::new(&[], mmio);
        cpu.registers.r[13] = 0x03007f00; // sp
        cpu.registers.r[15] = 0x08000000; // pc
        cpu.set_processor_mode(ProcessorMode::System);

        let mut trace: Vec<(u32, Instruction)> = Vec::new();

        loop {
            if let Ok((instr, state)) = cpu.tick() {
                trace.push((state.pc, instr));
            }
            cpu.mmio.tick_components();

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
                        println!(
                            "{:08X}: {:032b} -> {}",
                            faulting_pc,
                            cpu.mmio.read_u32(*faulting_pc),
                            faulting_instr
                        );
                    }
                }

                assert!(false, "Failed test: {}", cpu.read_register(&Register::R12));
            }

            if cpu.registers.r[15] == 0x08001d8c {
                break; // reached "all tests passed"
            }
        }

        assert!(true, "All tests passed");
    }
}
