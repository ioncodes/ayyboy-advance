fn dump_bios_function() {
    let lr = emu.read_register("lr");
    println(`Link Register: ${hex32(lr)}`);
    //println(`BIOS Function 0x${hex8(mmio.read_u8(lr - 2))}`);
}

fn setup() {
    let breakpoints = #{
        "0x8": "dump_bios_function"
    };
    breakpoints
}
