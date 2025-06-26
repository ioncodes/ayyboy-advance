fn fix_flash_read() {
    println(`r0: ${hex32(emu.read_register("r0"))}`);
    emu.write_register("r0", 1);
}

fn setup() {
    let breakpoints = #{
        "0x08032ea8": "fix_flash_read",
    };
    breakpoints
}
 