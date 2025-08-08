fn vibrate_on_money() {
    emu.vibrate(2000);
}

fn setup() {
    let breakpoints = #{
        "0x8076666": "vibrate_on_money",
    };
    breakpoints
}