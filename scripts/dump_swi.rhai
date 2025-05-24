fn dump_bios_function() {
    let lr = emu.read_register("lr");
    println(`BIOS Function 0x${hex8(emu.read_u8(lr - 2))}`);
}

fn setup() {
    let breakpoints = #{
        "0x8": "dump_bios_function"
    };
    breakpoints
}
