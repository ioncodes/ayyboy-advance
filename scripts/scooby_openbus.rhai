fn fix_openbus() {
    let pc = emu.read_register("pc");
    switch pc {
        0x0802CB0C => emu.write_register("r0", 0xE129F000),
        0x0802CB1C => emu.write_register("r0", 0x00000000)
        0x0802CB30 => emu.write_register("r1", 0xE129F000),
        0x0802CB40 => emu.write_register("r1", 0xE129F000),
        0x0802CB54 => emu.write_register("r0", 0xE129F000),
        0x0802CB5C => emu.write_register("r0", 0x71057105),
        0x0802CB68 => emu.write_register("r0", 0x00000000),
        0x0802D964 => emu.write_register("r0", 0xE129F000),
        0x0802D9D8 => emu.write_register("r0", 0x00000000),
        _ => println(`Unhandled openbus`),
    }
}

fn setup() {
    let breakpoints = #{
        "0x802CB0A": "fix_openbus",
        "0x802CB18": "fix_openbus",
        "0x802CB2E": "fix_openbus",
        "0x802CB3C": "fix_openbus",
        "0x802D962": "fix_openbus",
        "0x802CB52": "fix_openbus",
        "0x802CB5A": "fix_openbus",
        "0x802D9D6": "fix_openbus",
        "0x802cb66": "fix_openbus",
    };
    breakpoints
}
 