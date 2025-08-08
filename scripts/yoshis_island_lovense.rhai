fn vibrate_on_coin_pickup() {
    emu.vibrate(200);
}

fn setup() {
    let breakpoints = #{
        "0x804AAB8": "vibrate_on_coin_pickup",
    };
    breakpoints
}