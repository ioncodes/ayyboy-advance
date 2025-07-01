# ayyboy advance
ayyboy's big brother

|                                        Kirby - Nightmare in Dream Land                                         |                                       Wario Land 4                                        |                              Fire Emblem - The Sacred Stones                              |                                         Debugger                                          |
| :------------------------------------------------------------------------------------------------------------: | :---------------------------------------------------------------------------------------: | :---------------------------------------------------------------------------------------: | :---------------------------------------------------------------------------------------: |
| ![screenshot_20250531_001016](https://github.com/user-attachments/assets/570c7d4b-a593-4633-b7eb-474f98fd7ed8) | ![image](https://github.com/user-attachments/assets/fe3492f8-4f5e-4cb6-b795-cf8e3770a6e1) | ![image](https://github.com/user-attachments/assets/bf519a58-f5d7-4f00-8439-6909de2d83e4) | ![image](https://github.com/user-attachments/assets/ba13412a-61ee-486c-9bbc-96bc61e4cf44) |




## Setup
```bash
# place BIOS in external/gba_bios.bin
cargo build --release      # disables all logs at compile time
cargo build --profile dev  # logs all levels <=INFO by default, opt-level 3 for performance
```

## Usage
```
Usage: ayydbg.exe [OPTIONS] --rom <ROM>

Options:
      --trace            Enable trace-level logging (highest verbosity, incl. cpu dump and mmio events)
      --debug            Enable debug-level logging (mostly just cpu instructions)
      --script <SCRIPT>  Path to a custom script file
      --rom <ROM>        Path to the ROM file
  -h, --help             Print help
```

### Scripting
ayyboy advance support's [Rhai](https://rhai.rs/) scripts. These scripts allow you to execute a given "handler" on certain events, namely:
* Whenever the CPU executes an instruction at a given address
* Whenever the MMIO writes to or reads from a given address (janky implementation)

Refer to the [`scripts` folder](https://github.com/ioncodes/ayyboy-advance/tree/master/scripts) for examples. In particular, `dump_swi.rhai` is noteworthy as it logs every BIOS call.

### Screenshot Database
`rom-db` runs every `.zip` and `.gba` found in a given folder and takes a few screenshots every now and then. These are saved in `rom-db-ui/public/screenshots`:
```bash
# inside of rom-db-ui
npm install
npm run build # prod build is recommended for speed
npx serve@latest out
```

A snapshot of screenshots can be found in `external/screenshots.zip`. You can unpack these in the aforementioned folder.

## Compatibility
Currently passes [`armwrestler`](https://github.com/destoer/armwrestler-gba-fixed/tree/master) and a good number of [jsmolka's `gba-tests`](https://github.com/jsmolka/gba-tests). For game specific compatibility refer to the [screenshot database](https://ayyadvance.layle.dev/) (updated on milestones).
