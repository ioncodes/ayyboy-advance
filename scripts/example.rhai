fn dump_stack() {
    let sp = cpu.read_register("sp");
    println(`Stack @ ${hex32(sp)}:`);
    
    for i in -16..=16 {
        let addr = sp + (i * 4);
        let value = mmio.read_u32(addr);
        let offset = if i < 0 {
            "-" + padleft(`${-1 * i}`, "0", 2)
        } else {
            "+" + padleft(`${i}`, "0", 2)
        };
        println(`${hex32(addr)} [${offset}]: ${hex32(value)}`);
    }
}

fn print_opcode() {
    let dump = "";

    for i in 0..4 {
        let pc = 0x0801ad2a + i;
        let value = mmio.read_u8(pc);
        dump += `${hex8(value)}`;
    }

    println(`${dump}`);
}

fn setup() {
    let breakpoints = #{
        // "0x08008644": "dump_stack",
        // "0x0800865c": "dump_stack",
        // "0x08008588": "dump_stack",
        "0x0801ad2a": "print_opcode"
    };
    breakpoints
}
