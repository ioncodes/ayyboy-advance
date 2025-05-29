# ayyboy advance
ayyboy's big brother

## setup
```bash
# place BIOS in external/gba_bios.bin
cargo build --release      # disables all logs at compile time
cargo build --profile dev  # logs all levels <=INFO by default, opt-level 3 for performance
```

## usage
```
Usage: ayyboy-advance.exe [OPTIONS] --rom <ROM>

Options:
      --trace            Enable trace-level logging (highest verbosity, incl. cpu dump and mmio events)
      --debug            Enable debug-level logging (mostly just cpu instructions)
      --script <SCRIPT>  Path to a custom script file
      --rom <ROM>        Path to the ROM file
  -h, --help             Print help
```

### rom-db & rom-db-ui
`rom-db` runs every `.zip` and `.gba` found in a given folder and takes a few screenshots every now and then. These are saved in `rom-db/screenshots`. Move the contents to `target/rom-db` and then run:
```bash
# inside of rom-db-ui
npm install
npm run build # prod build is recommended for speed
npm run start
```

A snapshot of screenshots can be found in `external/screenshots.zip`.

## compatibility
passes:
* armwrestler

games:
* OpenLara
* Wolfenstein 3D
* some other games that use Mode 3-5
