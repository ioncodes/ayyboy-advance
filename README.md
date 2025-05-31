# ayyboy advance
ayyboy's big brother

|                                        Kirby - Nightmare in Dream Land                                         |                                         Debugger                                          |
| :------------------------------------------------------------------------------------------------------------: | :---------------------------------------------------------------------------------------: |
| ![screenshot_20250531_001016](https://github.com/user-attachments/assets/570c7d4b-a593-4633-b7eb-474f98fd7ed8) | ![image](https://github.com/user-attachments/assets/ba13412a-61ee-486c-9bbc-96bc61e4cf44) |

## setup
```bash
# place BIOS in external/gba_bios.bin
cargo build --release      # disables all logs at compile time
cargo build --profile dev  # logs all levels <=INFO by default, opt-level 3 for performance
```

## usage
```
Usage: ayydbg.exe [OPTIONS] --rom <ROM>

Options:
      --trace            Enable trace-level logging (highest verbosity, incl. cpu dump and mmio events)
      --debug            Enable debug-level logging (mostly just cpu instructions)
      --script <SCRIPT>  Path to a custom script file
      --rom <ROM>        Path to the ROM file
  -h, --help             Print help
```

### rom-db & rom-db-ui
`rom-db` runs every `.zip` and `.gba` found in a given folder and takes a few screenshots every now and then. These are saved in `rom-db-ui/screenshots`.:
```bash
# inside of rom-db-ui
npm install
npm run build # prod build is recommended for speed
npm run start
```

A snapshot of screenshots can be found in `external/screenshots.zip`. You can unpack these in the aforementioned folder.

## compatibility
passes:
* armwrestler

games:
* OpenLara
* Wolfenstein 3D
* Kirby - Nightmare in Dream Land
* More, but I'm too lazy to update atm
* some other games that use mode 3-5, maybe a few with mode 0 as well
