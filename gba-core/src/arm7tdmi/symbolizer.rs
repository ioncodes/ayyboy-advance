use goblin::Object;
use std::collections::HashMap;
use tracing::info;

pub struct Symbolizer {
    symbols: HashMap<u32, Vec<String>>,
}

impl Symbolizer {
    pub fn new(buffer: &[u8]) -> Symbolizer {
        let elf = match Object::parse(&buffer) {
            Ok(Object::Elf(elf)) => elf,
            _ => {
                // If the buffer is empty, we'll assume no ELF was found
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
                    .and_then(|name| (!name.is_empty()).then(|| (sym.st_value as u32, name.to_string())))
            })
            .fold(HashMap::new(), |mut map, (addr, name)| {
                map.entry(addr).or_default().push(name);
                map
            });

        info!(target: "symbols", "Loaded {} symbols", symbols.len());

        Symbolizer { symbols }
    }

    pub fn find(&self, addr: u32) -> Option<&Vec<String>> {
        self.symbols.get(&addr)
    }
}
