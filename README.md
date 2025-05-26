# ayyboy advance
ayyboy's big brother

## setup
```bash
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

## compatibility
passes:
* armwrestler

playable:
* OpenLara
* Wolfenstein 3D