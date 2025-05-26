fn dump_bios_function() {
    let BIOS_FUNC_LUT = #{
        "0x00": "SoftReset",
        "0x01": "RegisterRamReset",
        "0x02": "Halt",
        "0x03": "Stop/Sleep",
        "0x04": "IntrWait",
        "0x05": "VBlankIntrWait",
        "0x06": "Div",
        "0x07": "DivArm",
        "0x08": "Sqrt",
        "0x09": "ArcTan",
        "0x0a": "ArcTan2",
        "0x0b": "CpuSet",
        "0x0c": "CpuFastSet",
        "0x0d": "GetBiosChecksum",
        "0x0e": "BgAffineSet",
        "0x0f": "ObjAffineSet",
        "0x10": "BitUnPack",
        "0x11": "LZ77UnCompWram",
        "0x12": "LZ77UnCompVram",
        "0x13": "HuffUnComp",
        "0x14": "RLUnCompWram",
        "0x15": "RLUnCompVram",
        "0x16": "Diff8bitUnFilterWram",
        "0x17": "Diff8bitUnFilterVram",
        "0x18": "Diff16bitUnFilter",
        "0x19": "SoundBias",
        "0x1a": "SoundDriverInit",
        "0x1b": "SoundDriverMode",
        "0x1c": "SoundDriverMain",
        "0x1d": "SoundDriverVSync",
        "0x1e": "SoundChannelClear",
        "0x1f": "MidiKey2Freq",
        "0x20": "SoundWhatever0",
        "0x21": "SoundWhatever1",
        "0x22": "SoundWhatever2",
        "0x23": "SoundWhatever3",
        "0x24": "SoundWhatever4",
        "0x25": "MultiBoot",
        "0x26": "HardReset",
        "0x27": "CustomHalt",
        "0x28": "SoundDriverVSyncOff",
        "0x29": "SoundDriverVSyncOn",
        "0x2a": "SoundGetJumpList",
    };

    let lr = emu.read_register("lr");
    let bios_func = `0x${hex8(emu.read_u8(lr - 2))}`;
    println(`BIOS: ${BIOS_FUNC_LUT.get(bios_func)} (${bios_func})`);
}

fn setup() {
    let breakpoints = #{
        "0x8": "dump_bios_function"
    };
    breakpoints
}
