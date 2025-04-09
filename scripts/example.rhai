fn dump_stack() {
    let sp = cpu.read_register("sp");
    println(`Stack @ ${hex32(sp)}:`);
    
    for i in -16..16 {
        let addr = sp + (i * 4);
        let value = mmio.read_u32(addr);
        println(`${hex32(addr)}: ${hex32(value)}`);
    }
}

fn setup() {
    let breakpoints = #{
        "0x80026f6": "dump_stack",
    };
    breakpoints
}
