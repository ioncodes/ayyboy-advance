fn spoof_flash_chip_id() {
    let opcode = emu.read_u32(addr);
    let instr = disasm(opcode, emu.is_thumb());
    println(`Spoofing flash chip ID for instruction: ${instr}`);
    let operand = instr.sub_string(instr.index_of(" ") + 1, 2);
    emu.write_register(operand, 1);
}

fn setup() {
    let breakpoints = #{
        // memory rw breakpoints
        "0x0d000000": "spoof_flash_chip_id"
    };
    breakpoints
}