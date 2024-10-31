use goblin::Object;
use spdlog::{error, info};
use std::collections::HashMap;

pub struct Symbolizer {
    symbols: HashMap<u32, Vec<String>>,
}

impl Symbolizer {
    pub fn new(buffer: &[u8]) -> Symbolizer {
        let elf = match Object::parse(&buffer) {
            Ok(Object::Elf(elf)) => elf,
            _ => {
                error!("Input ELF is not a valid ELF file");
                return Symbolizer {
                    symbols: HashMap::new(),
                };
            }
        };

        let symbols: HashMap<u32, Vec<String>> = elf
            .syms
            .iter()
            .filter_map(|sym| {
                elf.strtab
                    .get_at(sym.st_name)
                    .filter(|name| !name.is_empty())
                    .map(|name| {
                        info!("Found symbol: {:08x} {}", sym.st_value, name);
                        (sym.st_value as u32, name.to_string())
                    })
            })
            .fold(HashMap::new(), |mut map, (addr, name)| {
                map.entry(addr).or_insert_with(Vec::new).push(name);
                map
            });

        info!("Loaded {} symbols", symbols.len());

        Symbolizer { symbols }
    }

    pub fn find(&self, addr: u32) -> Option<&Vec<String>> {
        self.symbols.get(&addr)
    }
}
